//! The tests of the lines the files say.

use super::*;

#[test]
fn standing_builders_are_read_a_line_each() {
    let file = "# at home\n\n192.168.4.46:22 ssh-ng://builder@home x86_64-linux - 2\n";
    assert_eq!(
        standing(file),
        [Standing {
            address: "192.168.4.46:22".to_owned(),
            line: "ssh-ng://builder@home x86_64-linux - 2".to_owned(),
        }]
    );
}

#[test]
fn a_lent_builder_is_named_for_its_big_work_alone() {
    let line = "ssh-ng://lent x86_64-linux /key 8 1 kvm,nixos-test,big-parallel,benchmark - KEY";
    assert_eq!(
        for_big_work(line),
        "ssh-ng://lent x86_64-linux /key 8 1 big-parallel,benchmark big-parallel KEY"
    );
    assert_eq!(
        for_big_work("ssh-ng://lent x86_64-linux - 4 1 big-parallel"),
        "ssh-ng://lent x86_64-linux - 4 1 big-parallel big-parallel -"
    );
    assert_eq!(
        for_big_work("ssh-ng://small x86_64-linux"),
        "ssh-ng://small x86_64-linux"
    );
}

#[test]
fn a_builder_named_for_its_big_work_keeps_what_it_already_requires() {
    assert_eq!(
        for_big_work("ssh-ng://vm x86_64-linux /key 4 0.5 big-parallel,own own KEY"),
        "ssh-ng://vm x86_64-linux /key 4 0.5 big-parallel,own own,big-parallel KEY"
    );
    assert_eq!(
        for_big_work("ssh-ng://vm x86_64-linux /key 4 1 big-parallel big-parallel KEY"),
        "ssh-ng://vm x86_64-linux /key 4 1 big-parallel big-parallel KEY"
    );
}

#[test]
fn the_file_says_those_answering_then_the_one_lent() {
    assert_eq!(text(["a", "b"].into_iter(), Some("lent")), "a\nb\nlent\n");
    assert_eq!(text(std::iter::empty(), None), "");
}
