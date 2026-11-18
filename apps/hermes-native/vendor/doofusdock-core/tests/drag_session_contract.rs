use doofusdock_core::*;

#[test]
fn drag_preview_is_transient_cancelled_without_target_and_revision_bound() {
    let mut session = DragSession::begin(7, vec![PanelId::from("panel-a")]).unwrap();
    assert_eq!(session.panels(), &[PanelId::from("panel-a")]);
    session.update_preview(7, Some("edge-left")).unwrap();
    assert_eq!(session.preview_target(), Some(&"edge-left"));
    assert_eq!(session.commit(7).unwrap(), Some("edge-left"));

    let mut cancelled = DragSession::begin(7, vec![PanelId::from("panel-a")]).unwrap();
    cancelled.update_preview(7, Some("edge-left")).unwrap();
    cancelled.cancel();
    assert_eq!(cancelled.commit(7).unwrap(), None);

    let mut stale = DragSession::begin(7, vec![PanelId::from("panel-a")]).unwrap();
    let error = stale.update_preview(8, Some("edge-right")).unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    assert_eq!(stale.commit(8).unwrap_err().code, ErrorCode::Conflict);
}

#[test]
fn drag_session_rejects_empty_and_duplicate_panel_payloads() {
    assert_eq!(
        DragSession::<String>::begin(0, Vec::new())
            .unwrap_err()
            .code,
        ErrorCode::ConstraintViolation
    );
    assert_eq!(
        DragSession::<String>::begin(0, vec![PanelId::from("panel-a"), PanelId::from("panel-a")])
            .unwrap_err()
            .code,
        ErrorCode::ConstraintViolation
    );
}

#[test]
fn typed_interruption_clears_preview_and_never_commits() {
    for reason in [
        DragCancelReason::Escape,
        DragCancelReason::PointerCaptureLost,
        DragCancelReason::HostFocusLost,
        DragCancelReason::ViewportClosed,
        DragCancelReason::SourceRemoved,
        DragCancelReason::TargetRemoved,
        DragCancelReason::RevisionConflict,
        DragCancelReason::PointerLeftClient,
        DragCancelReason::NativeMoveInterrupted,
    ] {
        let mut session = DragSession::begin(12, vec!["panel".into()]).unwrap();
        session.update_preview(12, Some("dock-target")).unwrap();
        session.cancel_with_reason(reason);
        assert_eq!(session.cancel_reason(), Some(reason));
        assert_eq!(session.preview_target(), None);
        assert_eq!(session.commit(12).unwrap(), None);
    }
}
