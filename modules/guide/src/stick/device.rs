//! A stick's whole disk, opened for writing by a person who isn't root:
//! on Linux through UDisks2, which asks polkit and hands back a file
//! descriptor, and on a Mac through authopen, which asks for an
//! administrator's password and hands one back over a socket. Whatever a
//! desktop mounted from the stick is unmounted first, since the disk is
//! opened exclusively. Reading what an old stick carries needs no
//! password: its partition is mounted only for reading, which polkit lets
//! the person at the machine do, as it doesn't for opening the disk.
//! Nothing else on a writer ever needs root.

use std::fs::File;
use std::path::Path;

use crate::Fault;

fn refused(device: &str, why: impl std::fmt::Display) -> Fault {
    Fault::Blocked {
        step: "write-stick",
        why: format!("{device} couldn't be opened for writing: {why}"),
    }
}

#[cfg(target_os = "linux")]
mod udisks {
    use std::collections::HashMap;
    use std::fs::File;
    use std::path::Path;

    use zbus::blocking::Connection;
    use zbus::zvariant::{OwnedFd, OwnedObjectPath, Value};

    use super::refused;
    use crate::Fault;

    const UDISKS: &str = "org.freedesktop.UDisks2";

    /// UDisks2's object for a block device, by its path.
    fn object(connection: &Connection, device: &str) -> Result<OwnedObjectPath, Fault> {
        let spec = HashMap::from([("path", Value::from(device))]);
        let options: HashMap<&str, Value> = HashMap::new();
        let reply = connection
            .call_method(
                Some(UDISKS),
                "/org/freedesktop/UDisks2/Manager",
                Some("org.freedesktop.UDisks2.Manager"),
                "ResolveDevice",
                &(spec, options),
            )
            .map_err(|why| refused(device, why))?;
        let objects: Vec<OwnedObjectPath> = reply
            .body()
            .deserialize()
            .map_err(|why| refused(device, why))?;
        objects
            .into_iter()
            .next()
            .ok_or_else(|| refused(device, "UDisks2 doesn't know it"))
    }

    /// Mounted only for reading while `read` looks inside, then
    /// unmounted.
    pub fn read_partition(device: &str, read: &mut dyn FnMut(&Path)) -> Result<(), Fault> {
        let unread = |why: zbus::Error| Fault::unreadable(device, why);
        let connection = Connection::system().map_err(unread)?;
        let partition = object(&connection, device)?;
        let options = HashMap::from([("options", Value::from("ro"))]);
        let reply = connection
            .call_method(
                Some(UDISKS),
                &partition,
                Some("org.freedesktop.UDisks2.Filesystem"),
                "Mount",
                &(options,),
            )
            .map_err(unread)?;
        let mounted: String = reply.body().deserialize().map_err(unread)?;
        read(Path::new(&mounted));
        let options: HashMap<&str, Value> = HashMap::new();
        connection
            .call_method(
                Some(UDISKS),
                &partition,
                Some("org.freedesktop.UDisks2.Filesystem"),
                "Unmount",
                &(options,),
            )
            .map_err(unread)?;
        Ok(())
    }

    pub fn open(device: &str, mounted: &[String]) -> Result<File, Fault> {
        let connection = Connection::system().map_err(|why| refused(device, why))?;
        for partition in mounted {
            let options: HashMap<&str, Value> = HashMap::new();
            connection
                .call_method(
                    Some(UDISKS),
                    &object(&connection, partition)?,
                    Some("org.freedesktop.UDisks2.Filesystem"),
                    "Unmount",
                    &(options,),
                )
                .map_err(|why| refused(partition, why))?;
        }
        // Exclusive, so nothing mounts it while it's written; no O_SYNC,
        // since the writer syncs once at the end and reads it all back.
        let options = HashMap::from([("flags", Value::from(libc::O_EXCL | libc::O_CLOEXEC))]);
        let reply = connection
            .call_method(
                Some(UDISKS),
                &object(&connection, device)?,
                Some("org.freedesktop.UDisks2.Block"),
                "OpenDevice",
                &("rw", options),
            )
            .map_err(|why| refused(device, why))?;
        let fd: OwnedFd = reply
            .body()
            .deserialize()
            .map_err(|why| refused(device, why))?;
        Ok(File::from(std::os::fd::OwnedFd::from(fd)))
    }
}

#[cfg(target_os = "macos")]
mod authopen {
    use std::fs::File;
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::net::UnixStream;
    use std::process::{Command, Stdio};

    use super::refused;
    use crate::Fault;

    /// The one file descriptor authopen sends over the socket.
    fn received(socket: &UnixStream, device: &str) -> Result<OwnedFd, Fault> {
        use std::os::fd::AsRawFd;
        let mut byte = [0u8; 1];
        let mut io = libc::iovec {
            iov_base: byte.as_mut_ptr().cast(),
            iov_len: 1,
        };
        let int = u32::try_from(size_of::<libc::c_int>()).expect("an int is four bytes");
        // SAFETY: CMSG_SPACE only computes a size.
        let space = unsafe { libc::CMSG_SPACE(int) };
        let mut control = vec![0u8; space as usize];
        // SAFETY: a zeroed msghdr is valid, and every pointer set in it
        // points at a buffer that outlives the call.
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_iov = &raw mut io;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = space;
        // SAFETY: the socket is open and the message's buffers are ours.
        if unsafe { libc::recvmsg(socket.as_raw_fd(), &raw mut message, 0) } < 0 {
            return Err(refused(device, std::io::Error::last_os_error()));
        }
        // SAFETY: CMSG_FIRSTHDR reads only the message recvmsg filled.
        let header = unsafe { libc::CMSG_FIRSTHDR(&raw const message) };
        // SAFETY: the header is checked for being there and for carrying
        // rights before its data is read as a descriptor.
        unsafe {
            if header.is_null()
                || (*header).cmsg_level != libc::SOL_SOCKET
                || (*header).cmsg_type != libc::SCM_RIGHTS
            {
                return Err(refused(device, "authopen sent no file descriptor"));
            }
            let fd = std::ptr::read_unaligned(libc::CMSG_DATA(header).cast::<libc::c_int>());
            Ok(OwnedFd::from_raw_fd(fd))
        }
    }

    pub fn open(device: &str, _mounted: &[String]) -> Result<File, Fault> {
        // A Mac's whole disk is unmounted by diskutil, and written through
        // its raw device, which goes past the cache.
        let unmounted = Command::new("/usr/sbin/diskutil")
            .args(["unmountDisk", device])
            .status()
            .map_err(|why| refused(device, why))?;
        if !unmounted.success() {
            return Err(refused(device, "diskutil couldn't unmount it"));
        }
        let raw = device.replacen("/dev/disk", "/dev/rdisk", 1);
        let (ours, theirs) = UnixStream::pair().map_err(|why| refused(device, why))?;
        let mut child = Command::new("/usr/libexec/authopen")
            .args(["-stdoutpipe", "-o", &libc::O_RDWR.to_string(), &raw])
            .stdout(Stdio::from(OwnedFd::from(theirs)))
            .spawn()
            .map_err(|why| refused(device, why))?;
        let fd = received(&ours, device);
        let status = child.wait().map_err(|why| refused(device, why))?;
        if !status.success() {
            return Err(refused(device, "authopen was refused"));
        }
        Ok(File::from(fd?))
    }
}

/// A stick's partition, mounted only for reading while `read` looks
/// inside it. A Mac mounts a stick's partitions itself, so there it's read
/// where it's mounted.
pub fn read_partition(device: &str, read: &mut dyn FnMut(&Path)) -> Result<(), Fault> {
    #[cfg(target_os = "linux")]
    return udisks::read_partition(device, read);
    #[cfg(not(target_os = "linux"))]
    {
        let _ = read;
        Err(Fault::unreadable(
            device,
            "this system reads a stick where it mounts it",
        ))
    }
}

/// The stick's whole disk, for reading and writing, once whatever was
/// mounted from it (`mounted`, its partitions' devices) is unmounted.
pub fn open(device: &str, mounted: &[String]) -> Result<File, Fault> {
    #[cfg(target_os = "linux")]
    return udisks::open(device, mounted);
    #[cfg(target_os = "macos")]
    return authopen::open(device, mounted);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = mounted;
        Err(refused(device, "this system has no way to open it"))
    }
}
