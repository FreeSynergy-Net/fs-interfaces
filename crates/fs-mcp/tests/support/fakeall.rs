//! A program that implements EVERY method of `tasks.capnp` and answers with empty (default) data and a few list elements, so
//! that the generated command code walks real capnp messages of every result type. Test helpers; no pattern of their own.
#![allow(dead_code)]

use std::future::{Future, ready};

use fs_interfaces::tasks_capnp::{boards, items, tasks};
use fs_types::types_capnp::type_catalog;

pub struct All;
struct AllItems;
struct AllBoards;
struct AllTypes;

macro_rules! plain {
    ($name:ident, $p:ty, $r:ty) => {
        fn $name(
            self: capnp::capability::Rc<Self>,
            _: $p,
            _: $r,
        ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
            ready(Ok(()))
        }
    };
}

impl items::Server for AllItems {
    plain!(create, items::CreateParams, items::CreateResults);
    plain!(edit, items::EditParams, items::EditResults);
    plain!(assign, items::AssignParams, items::AssignResults);
    plain!(
        create_relation,
        items::CreateRelationParams,
        items::CreateRelationResults
    );
    plain!(
        assign_label,
        items::AssignLabelParams,
        items::AssignLabelResults
    );
    plain!(get, items::GetParams, items::GetResults);
    plain!(get_by_uid, items::GetByUidParams, items::GetByUidResults);
    plain!(delete, items::DeleteParams, items::DeleteResults);
    plain!(query, items::QueryParams, items::QueryResults);
    plain!(import_ics, items::ImportIcsParams, items::ImportIcsResults);
    plain!(export_ics, items::ExportIcsParams, items::ExportIcsResults);
    plain!(edit_if, items::EditIfParams, items::EditIfResults);
    plain!(readiness, items::ReadinessParams, items::ReadinessResults);
    plain!(
        create_relation_with_gap,
        items::CreateRelationWithGapParams,
        items::CreateRelationWithGapResults
    );
    plain!(claim, items::ClaimParams, items::ClaimResults);
    plain!(release, items::ReleaseParams, items::ReleaseResults);

    fn list(
        self: capnp::capability::Rc<Self>,
        _: items::ListParams,
        mut r: items::ListResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        r.get().init_page().init_items(2);
        ready(Ok(()))
    }

    fn blocked_by(
        self: capnp::capability::Rc<Self>,
        _: items::BlockedByParams,
        mut r: items::BlockedByResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        r.get().init_tasks(2);
        ready(Ok(()))
    }

    fn dependents(
        self: capnp::capability::Rc<Self>,
        _: items::DependentsParams,
        mut r: items::DependentsResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        r.get().init_tasks(1);
        ready(Ok(()))
    }
}

impl boards::Server for AllBoards {
    plain!(create, boards::CreateParams, boards::CreateResults);
    plain!(layout, boards::LayoutParams, boards::LayoutResults);
    plain!(
        edit_columns,
        boards::EditColumnsParams,
        boards::EditColumnsResults
    );
    plain!(move_card, boards::MoveCardParams, boards::MoveCardResults);

    fn list(
        self: capnp::capability::Rc<Self>,
        _: boards::ListParams,
        mut r: boards::ListResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        r.get().init_boards(2);
        ready(Ok(()))
    }
}

impl type_catalog::Server for AllTypes {
    plain!(
        describe,
        type_catalog::DescribeParams,
        type_catalog::DescribeResults
    );
}

impl tasks::Server for All {
    plain!(
        capabilities,
        tasks::CapabilitiesParams,
        tasks::CapabilitiesResults
    );
    plain!(
        support_report,
        tasks::SupportReportParams,
        tasks::SupportReportResults
    );

    fn items(
        self: capnp::capability::Rc<Self>,
        _: tasks::ItemsParams,
        mut r: tasks::ItemsResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        r.get().set_items(capnp_rpc::new_client(AllItems));
        ready(Ok(()))
    }

    fn boards(
        self: capnp::capability::Rc<Self>,
        _: tasks::BoardsParams,
        mut r: tasks::BoardsResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        r.get().set_boards(capnp_rpc::new_client(AllBoards));
        ready(Ok(()))
    }

    fn types(
        self: capnp::capability::Rc<Self>,
        _: tasks::TypesParams,
        mut r: tasks::TypesResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        r.get().set_catalog(capnp_rpc::new_client(AllTypes));
        ready(Ok(()))
    }
}

pub fn client() -> tasks::Client {
    capnp_rpc::new_client(All)
}
