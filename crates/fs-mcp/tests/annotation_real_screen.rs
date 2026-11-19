//! What the marks `$Cli.secret` of the REAL contracts do in `fs-mcp`, as it is today: which commands are no tool, with the
//! mark files of the program and without any mark. Pattern: none (tests).

mod support;

use fs_mcp::{Blocklist, MarkFiles, Target};
use support::real::{contracts, generate_with, real_catalog};

/// Commands that take or give a field marked `$Cli.secret`, and `Respond.submit` split by the member of its `Credential`.
const SECRET_COMMANDS: [(&str, &str, &str); 5] = [
    ("messenger", "Accounts", "add"),
    ("streaming", "Streams", "create"),
    ("streaming", "Streams", "rotateKey"),
    ("streaming", "Streams", "addTarget"),
    ("streaming", "Streams", "setTargetKey"),
];
const SUBMIT_MEMBERS: [&str; 5] = ["none", "person", "token", "proof", "open"];

fn is_tool(c: &fs_mcp::Catalog, t: &Target) -> bool {
    c.tools.iter().any(|x| &x.source == t)
}

#[test]
fn with_the_marks_of_the_program_no_command_with_a_secret_is_a_tool_and_respond_submit_has_no_member_tool()
 {
    let c = real_catalog();
    for (f, o, m) in SECRET_COMMANDS {
        assert!(!is_tool(&c, &Target::item(f, o, m)), "{f}/{o}.{m}");
    }
    for m in SUBMIT_MEMBERS {
        assert!(
            !is_tool(&c, &Target::item("surveys", "Credential", m)),
            "Credential.{m}"
        );
        // and each member is named as left out, not forgotten
        assert!(
            c.left_out
                .iter()
                .any(|l| l.target == Target::item("surveys", "Credential", m)),
            "Credential.{m} is not listed as left out"
        );
    }
    assert!(!is_tool(&c, &Target::item("surveys", "Respond", "submit")));
}

#[test]
fn without_any_mark_the_same_commands_stay_out() {
    let bare = generate_with(
        &contracts(),
        &MarkFiles::from_sources(&[]).unwrap(),
        Blocklist::empty(),
    );
    for (f, o, m) in SECRET_COMMANDS {
        assert!(!is_tool(&bare, &Target::item(f, o, m)), "{f}/{o}.{m}");
    }
    // the member `token` of the credential is out in every case; `Respond.submit` is never a tool as a whole
    assert!(!is_tool(
        &bare,
        &Target::item("surveys", "Credential", "token")
    ));
    assert!(!is_tool(
        &bare,
        &Target::item("surveys", "Respond", "submit")
    ));
}
