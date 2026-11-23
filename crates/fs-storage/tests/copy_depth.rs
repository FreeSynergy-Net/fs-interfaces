//! A copy into its own subtree (the mutant `NoCycle`: the cycle check is gone) must end with an error and never run
//! away: no stack overflow, no endless growth. The copy runs on a thread with a small stack, so a missing depth
//! limit shows as a failed test and not as a lost process.

use fs_storage::{
    Ctl, EntryId, Error, FakeStorage, Fault, Kind, OnConflict, StorageProvider, sample,
};

const SMALL_STACK: usize = 256 * 1024;

async fn copy_into_own_subtree(fault: Option<Fault>) -> Result<(), Error> {
    let s = sample();
    let mut fake = FakeStorage::new(&s.anna);
    if let Some(f) = fault {
        fake = fake.with_fault(f);
    }
    let roots = fake.roots().await?;
    let root: EntryId = roots
        .iter()
        .find(|r| r.owner == s.anna)
        .expect("Anna has a root")
        .id
        .clone();
    let a = fake
        .create(&root, "a-copy-depth", Kind::Folder, OnConflict::Fail)
        .await?;
    let b = fake
        .create(&a.id, "b", Kind::Folder, OnConflict::Fail)
        .await?;
    fake.create(&b.id, "c", Kind::Folder, OnConflict::Fail)
        .await?;
    fake.copy_to(&a.id, &b.id, OnConflict::KeepBoth, &Ctl::default())
        .await
        .map(|_| ())
}

fn on_small_stack(fault: Option<Fault>) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("runtime")
                .block_on(copy_into_own_subtree(fault))
        })
        .expect("thread")
        .join()
        .expect("the copy must not overflow the stack")
}

#[test]
fn the_fake_refuses_a_copy_into_itself() {
    assert!(matches!(on_small_stack(None), Err(Error::Cycle(_))));
}

#[test]
fn without_the_cycle_check_the_copy_ends_with_an_error_not_a_crash() {
    assert!(matches!(
        on_small_stack(Some(Fault::NoCycle)),
        Err(Error::BackendFailed(_))
    ));
}
