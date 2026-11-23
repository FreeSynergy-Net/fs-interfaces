//! Small checks of the operator part that need no conformance run: the single user, the scope order.

use fs_group_directory::FakeGroups;
use std::sync::Arc;

#[tokio::test]
async fn the_single_user_is_the_operator() {
    let people = Arc::new(fs_group_directory::FakePeople::new());
    let me = people.human("Alex Solo");
    let fake = FakeGroups::with_people(people, me.clone()).with_operator(&me);
    let list = fs_group_directory::groups::GroupGovernance::operators(&fake)
        .await
        .expect("the list is open");
    assert_eq!(list.persons, vec![me]);
    assert!(list.group.is_some());
}

#[test]
fn only_the_scope_server_covers_server() {
    use fs_group_directory::groups::Scope;
    assert!(!Scope::Project.covers(Scope::Server));
    assert!(!Scope::AllGroups.covers(Scope::Server));
    assert!(Scope::Server.covers(Scope::Server));
    assert!(!Scope::Server.covers(Scope::Project));
}
