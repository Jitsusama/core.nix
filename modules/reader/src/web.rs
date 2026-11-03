//! Pictures on the web. A page names them; the first set draws each as a
//! box saying what it shows, and one thread fetches them, without cookies
//! or a referrer, into the reader's cache. When the last of a batch has
//! come, the page is set again with them, as a save sets it, so the view
//! stays on its words. A picture fetched once is read from the cache after,
//! so it is there from the first frame.
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, mpsc};
use std::time::Duration;

/// The most a picture may weigh; anything larger stays a box.
const AT_MOST: u64 = 20 * 1024 * 1024;
/// How long one picture may take, start to end.
const TAKES_AT_MOST: Duration = Duration::from_secs(15);

struct Fetcher {
    wanted: mpsc::Sender<String>,
    /// Every picture asked for, so one that failed isn't asked again each
    /// time the page sets.
    asked: Mutex<HashSet<String>>,
}

static FETCHER: OnceLock<Fetcher> = OnceLock::new();
static FETCHED: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();
/// A batch came before anyone said what to do with it.
static PENDING: AtomicBool = AtomicBool::new(false);

/// Says what to do once a batch of pictures has come: set the page again.
/// A batch that came already is done at once.
pub fn when_fetched(then: impl Fn() + Send + Sync + 'static) {
    let _ = FETCHED.set(Box::new(then));
    if PENDING.swap(false, Ordering::SeqCst) {
        fetched();
    }
}

fn fetched() {
    if let Some(then) = FETCHED.get() {
        then();
        return;
    }
    PENDING.store(true, Ordering::SeqCst);
    // Said between the look and the flag: do it here instead.
    if FETCHED.get().is_some() && PENDING.swap(false, Ordering::SeqCst) {
        fetched();
    }
}

/// Whether a picture's address is on the web.
pub fn is_web(src: &str) -> bool {
    src.starts_with("https://") || src.starts_with("http://")
}

/// The file a picture's bytes are kept in, named for its address.
fn kept_at(src: &str) -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    let mut name = DefaultHasher::new();
    src.hash(&mut name);
    Some(
        cache
            .join("reader")
            .join("web")
            .join(format!("{:016x}", name.finish())),
    )
}

/// A picture fetched before, if the cache has it.
pub fn cached(src: &str) -> Option<Vec<u8>> {
    std::fs::read(kept_at(src)?).ok()
}

/// Asks for a picture to be fetched, once.
pub fn want(src: &str) {
    let fetcher = FETCHER.get_or_init(|| {
        let (wanted, asked_for) = mpsc::channel();
        let _ = std::thread::Builder::new()
            .name("web".into())
            .spawn(move || fetch_all(&asked_for));
        Fetcher {
            wanted,
            asked: Mutex::new(HashSet::new()),
        }
    });
    let fresh = fetcher
        .asked
        .lock()
        .is_ok_and(|mut asked| asked.insert(src.to_string()));
    if fresh {
        let _ = fetcher.wanted.send(src.to_string());
    }
}

fn fetch_all(asked_for: &mpsc::Receiver<String>) {
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(TAKES_AT_MOST))
        .tls_config(tls)
        .build();
    let agent = ureq::Agent::new_with_config(config);
    while let Ok(first) = asked_for.recv() {
        let mut came = 0;
        let mut next = Some(first);
        // Everything asked for while a batch is fetched joins it, so the
        // page sets again once for all of them.
        while let Some(src) = next {
            match fetch(&agent, &src) {
                Ok(()) => came += 1,
                Err(error) => eprintln!("reader: can't fetch {src}: {error}"),
            }
            next = asked_for.try_recv().ok();
        }
        if came > 0 {
            fetched();
        }
    }
}

fn fetch(agent: &ureq::Agent, src: &str) -> Result<(), String> {
    let kept = kept_at(src).ok_or("no cache folder")?;
    let mut answer = agent.get(src).call().map_err(|e| e.to_string())?;
    let bytes = answer
        .body_mut()
        .with_config()
        .limit(AT_MOST)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    if crate::walk::picture_kind(&bytes).is_none() {
        return Err("not a picture the reader can set".into());
    }
    if let Some(folder) = kept.parent() {
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    }
    let fresh = kept.with_extension(format!("{}.part", std::process::id()));
    std::fs::write(&fresh, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&fresh, &kept).map_err(|e| e.to_string())
}
