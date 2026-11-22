#!/usr/bin/env python3
"""Build a conservative implementation overlay for the fixed 499-row scope."""

from __future__ import annotations

import argparse
import csv
import json
import os
import tempfile
from collections import Counter
from datetime import date
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/planning/ACCEPTANCE_MATRIX.csv"
DEFAULT_OUTPUT = ROOT / "apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv"
PIN = "8783054ef6cbafd86f10892241436862c1171a66"

EVIDENCE: dict[str, dict[str, object]] = {
    "PAR-0002": {
        "state": "partial",
        "level": "native_launch_and_layout_isolation_tests",
        "refs": [
            "apps/hermes-native/src/app.rs#fn spawn_native_window",
            "apps/hermes-native/src/app.rs#failed_native_window_startup_preserves_the_source_session",
            "apps/hermes-native/src/native_window.rs#matching_one_use_receipt_is_accepted",
            "apps/hermes-native/src/native_window.rs#mismatched_id_or_nonce_is_rejected",
            "apps/hermes-native/src/layout.rs#pub fn load_window(window_id: u64)",
            "apps/hermes-native/src/layout.rs#separate_window_uses_distinct_layout_and_preferences_but_inherits_gateway_url",
            "apps/hermes-native/src/main.rs#window_launch_arguments_preserve_route_and_profile",
        ],
        "gaps": ["native Windows process/window launch and close not physically verified", "cross-window session/event synchronization", "window close and restore lifecycle", "source-paired Windows behavior and visuals"],
    },
    "PAR-0010": {
        "state": "partial",
        "level": "headless_session_move_and_launch_lifecycle",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 172",
            "apps/hermes-native/src/app.rs#move_current_session_closes_source_only_after_readiness_receipt",
            "apps/hermes-native/src/app.rs#native_window_readiness_timeout_preserves_the_source_session",
            "apps/hermes-native/src/app.rs#failed_native_window_startup_preserves_the_source_session",
            "apps/hermes-native/src/app.rs#source_session_becoming_active_during_launch_prevents_move_after_ready_receipt",
            "apps/hermes-native/src/app.rs#inactive_source_session_cached_draft_prevents_move_after_ready_receipt",
            "apps/hermes-native/src/native_window.rs#matching_one_use_receipt_is_accepted",
            "apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs#typed_app_action_and_human_widget_share_the_registered_handler",
        ],
        "gaps": ["actual Windows process/window move not physically exercised", "source-paired Windows visual acceptance", "cross-window session/event synchronization remains a separate requirement"],
    },
    "PAR-0007": {
        "state": "partial",
        "level": "headless_pointer_and_dock_behavior",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/renderer/drag-session.ts#radialPosition",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/renderer/tree-split.tsx#startSash",
            "apps/hermes-native/src/app.rs#fn compute_drop_edge",
            "apps/hermes-native/src/app.rs#fn compute_drop_preview",
            "apps/hermes-native/src/app.rs#fn panel_drop_operation",
            "apps/hermes-native/src/app.rs#canonical_panel_drop_commits_a_tab_operation_and_persists_it",
            "apps/hermes-native/src/app.rs#panel_drop_regions_match_pinned_radial_geometry",
            "apps/hermes-native/src/app.rs#split_resize_preserves_weight_pair_and_clamps_each_side",
        ],
        "gaps": ["source-level whole-zone FancyZones highlights, Shift spanning/merging, and group-level range selection", "four-pixel pointer engage threshold, source radial proximity hit-testing across adjacent zones, and complete split edge cases", "physical Windows drag/resize behavior, authenticated runtime and source-paired visuals/acceptance remain unverified"],
    },
    "PAR-0008": {
        "state": "partial",
        "level": "headless_pointer_and_dock_behavior",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/renderer/drag-session.ts#startPaneDrag",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/renderer/tree-group.tsx#ZoneDropOverlay",
            "apps/hermes-native/src/app.rs#fn begin_panel_drag",
            "apps/hermes-native/src/app.rs#fn update_panel_drop_target",
            "apps/hermes-native/src/app.rs#fn panel_drop_operation",
            "apps/hermes-native/src/app.rs#fn commit_panel_drop",
            "apps/hermes-native/src/app.rs#canonical_panel_drop_commits_a_tab_operation_and_persists_it",
            "apps/hermes-native/src/app.rs#panel_drop_regions_match_pinned_radial_geometry",
        ],
        "gaps": ["drop hints currently target individual rendered panels rather than every eligible source zone", "tab-strip insertion caret, tear-off-slack transition, Shift multi-zone drop and moving selected tab blocks are not implemented", "physical Windows drag behavior, invalid-area cursor semantics and source-paired visuals/acceptance remain unverified"],
    },
    "PAR-0011": {
        "state": "partial",
        "level": "headless_profile_fenced_session_transcript_and_repo_discovery",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/store/session-sync.ts#onSessionsChanged",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/transcript-sync.ts#broadcastTranscriptChanged",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-transcript-peer-sync.ts#useTranscriptPeerSync",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/contrib/hooks/use-background-sync.ts#SESSIONS_LIST_TICK_GAP_MS",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/contrib/hooks/use-background-sync.ts#rehydrateLiveSessionStatuses",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/contrib/hooks/use-background-sync.ts#LIVE_SESSION_STATUS_BACKSTOP_INTERVAL_MS",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/contrib/hooks/use-background-sync.ts#reconcileActiveTranscript",
            ".staging/hermes-pinned-878305/apps/desktop/src/api/sessions.ts#getLatestSessionMessages",
            "apps/hermes-native/docs/PLAN.md#Revision 176",
            "apps/hermes-native/docs/PLAN.md#Revision 178",
            "apps/hermes-native/docs/PLAN.md#Revision 183",
            "apps/hermes-native/src/app.rs#fn handle_server_event",
            "apps/hermes-native/src/app.rs#fn handle_sessions_refreshed",
            "apps/hermes-native/src/app.rs#fn refresh_active_transcript_after_change",
            "apps/hermes-native/src/app.rs#fn settle_active_transcript_refresh",
            "apps/hermes-native/src/app.rs#fn refresh_peer_transcript_targets",
            "apps/hermes-native/src/app.rs#fn settle_peer_transcript_refresh",
            "apps/hermes-native/src/app.rs#fn broadcast_local_transcript_changed",
            "apps/hermes-native/src/app.rs#fn request_active_session_statuses",
            "apps/hermes-native/src/app.rs#fn settle_active_session_statuses",
            "apps/hermes-native/src/app.rs#fn recover_after_focus",
            "apps/hermes-native/src/app.rs#fn load_project_list",
            "apps/hermes-native/src/app.rs#fn settle_project_list",
            "apps/hermes-native/src/app.rs#fn project_list_active_id",
            "apps/hermes-native/src/app.rs#project_list_is_profile_scoped_and_keeps_its_active_pointer_separate_from_tree",
            "apps/hermes-native/src/app.rs#project_list_rejects_stale_generation_and_preserves_cache_on_failure",
            "apps/hermes-native/src/app.rs#projects_changed_refreshes_only_the_matching_profile_list",
            "apps/hermes-native/src/app.rs#show_all_project_sessions_control_refreshes_tree_with_source_preview_limits",
            "apps/hermes-native/src/app.rs#fn sessions_content",
            "apps/hermes-native/src/app.rs#fn load_project_session_colors",
            "apps/hermes-native/src/app.rs#fn settle_project_tree",
            "apps/hermes-native/src/app.rs#fn enter_project_sessions",
            "apps/hermes-native/src/app.rs#fn request_entered_project_sessions",
            "apps/hermes-native/src/app.rs#fn settle_project_sessions",
            "apps/hermes-native/src/app.rs#fn filter_removed_project_sessions",
            "apps/hermes-native/src/app.rs#fn request_project_preview",
            "apps/hermes-native/src/app.rs#fn settle_project_preview",
            "apps/hermes-native/src/app.rs#fn reveal_more_project_preview",
            "apps/hermes-native/src/app.rs#all_profile_project_entry_uses_aggregate_rows_without_synthetic_rpc",
            "apps/hermes-native/src/app.rs#project_show_all_hydrates_profile_scope_and_reveals_fifty_rows_at_a_time",
            "apps/hermes-native/src/app.rs#project_show_all_controls_and_fifty_row_paging_render_accessibly",
            "apps/hermes-native/src/app.rs#show_all_project_sessions_control_refreshes_tree_with_source_preview_limits",
            "apps/hermes-native/src/app.rs#show_all_project_sessions_uses_two_thousand_preview_for_all_profiles",
            "apps/hermes-native/src/layout.rs#show_all_project_sessions_preference_survives_workspace_reload",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/sidebar/projects/overview-row.tsx#showAll",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/sidebar/projects/model.ts#PROJECT_SESSION_PAGE",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/sidebar/filter-menu.tsx#showAllSessions",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/projects.ts#projectTreePreviewLimit",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/layout.ts#SIDEBAR_SHOW_ALL_SESSIONS_STORAGE_KEY",
            "apps/hermes-native/src/app.rs#fn refresh_project_tree_colors",
            "apps/hermes-native/src/app.rs#project_change_from_any_profile_invalidates_the_all_profile_tree",
            "apps/hermes-native/src/app.rs#fn recover_after_focus",
            "apps/hermes-native/src/app.rs#project_tree_results_are_profile_connection_and_generation_fenced",
            "apps/hermes-native/src/app.rs#project_sessions_reject_stale_navigation_and_keep_cached_project_on_error",
            "apps/hermes-native/src/app.rs#changed_project_overview_refetches_only_the_entered_project",
            "apps/hermes-native/src/app.rs#active_status_snapshot_is_profile_scoped_and_cannot_overwrite_newer_events",
            "apps/hermes-native/src/app.rs#active_session_waiting_state_is_exposed_to_session_row_locators",
            "apps/hermes-native/src/app.rs#projects_changed_invalidates_only_its_profile_and_keeps_cached_colors",
            "apps/hermes-native/src/app.rs#peer_transcript_refresh_targets_open_profile_scoped_tiles_and_fences_stale_receipts",
            "apps/hermes-native/src/app.rs#sessions_changed_refreshes_without_an_active_session_and_coalesces_bursts",
            "apps/hermes-native/src/app.rs#sessions_refreshed_requires_matching_request_profile_and_current_gateway",
            "apps/hermes-native/src/app.rs#active_transcript_refresh_is_request_fenced_and_preserves_draft_and_newer_events",
            "apps/hermes-native/src/app.rs#refreshed_transcript_tail_keeps_history_and_rejects_unprovable_or_empty_pages",
            "apps/hermes-native/src/gateway.rs#pub fn refresh_sessions",
            "apps/hermes-native/src/gateway.rs#fn parse_active_session_statuses",
            "apps/hermes-native/src/gateway.rs#active_session_status_read_uses_pinned_rpc_and_explicit_profile_scope",
            "apps/hermes-native/src/gateway.rs#active_session_status_parser_requires_bounded_unique_profile_rows",
            "apps/hermes-native/src/gateway.rs#fn parse_project_tree",
            "apps/hermes-native/src/gateway.rs#fn parse_project_list",
            "apps/hermes-native/src/gateway.rs#project_list_parser_preserves_source_metadata_and_rejects_invalid_active_ids",
            "apps/hermes-native/src/gateway.rs#project_list_read_uses_pinned_rpc_and_explicit_profile_scope",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/projects.ts#refreshProjects",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/sidebar/projects/project-menu.tsx#useProjectActions",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/projects.ts#deleteProject",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/sidebar/projects/project-menu.tsx#removeAuto",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/layout.ts#dismissAutoProject",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/layout.ts#restoreAutoProject",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/layout.ts#filterVisibleProjects",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/layout-dismissed-projects.test.ts#restores a dismissed auto project (Undo round-trip)",
            ".staging/hermes-pinned-878305/apps/desktop/src/types/hermes.ts#ProjectInfo",
            "apps/hermes-native/docs/PLAN.md#Revision 192",
            "apps/hermes-native/docs/PLAN.md#Revision 193",
            "apps/hermes-native/src/layout.rs#pub fn set_auto_project_dismissed",
            "apps/hermes-native/src/layout.rs#pub fn refresh_auto_project_dismissals",
            "apps/hermes-native/src/layout.rs#auto_project_dismissals_are_shared_across_window_workspaces",
            "apps/hermes-native/src/layout.rs#auto_project_state_recovers_expired_lock_and_rejects_invalid_state_atomically",
            "apps/hermes-native/src/layout.rs#concurrent_auto_project_dismissal_updates_do_not_clobber",
            "apps/hermes-native/src/app.rs#fn project_delete_dialog",
            "apps/hermes-native/src/app.rs#fn request_project_delete",
            "apps/hermes-native/src/app.rs#fn settle_project_delete",
            "apps/hermes-native/src/app.rs#fn dismiss_auto_project",
            "apps/hermes-native/src/app.rs#fn restore_auto_project",
            "apps/hermes-native/src/app.rs#fn poll_shared_auto_project_dismissals",
            "apps/hermes-native/src/app.rs#auto_project_hide_is_local_persistent_and_exits_scoped_detail",
            "apps/hermes-native/src/app.rs#auto_project_hide_and_undo_controls_are_accessible",
            "apps/hermes-native/src/app.rs#dismissed_auto_project_filter_never_hides_explicit_id_collisions",
            "apps/hermes-native/src/app.rs#explicit_project_delete_is_profile_scoped_optimistic_and_rolls_back",
            "apps/hermes-native/src/gateway.rs#project_delete_uses_explicit_profile_rpc_and_validates_receipt",
            "apps/hermes-native/src/gateway.rs#project_delete_rejects_synthetic_all_profile_and_invalid_ids",
            "apps/hermes-native/docs/PLAN.md#Revision 191",
            "apps/hermes-native/src/gateway.rs#fn validate_all_profile_project_tree",
            "apps/hermes-native/src/gateway.rs#fn load_all_profile_project_tree_http",
            "apps/hermes-native/src/gateway.rs#all_profile_project_tree_uses_profile_aggregate_and_preserves_owners_and_errors",
            "apps/hermes-native/src/gateway.rs#project_sessions_read_uses_exact_project_and_profile_scope",
            "apps/hermes-native/src/gateway.rs#project_tree_parser_preserves_backend_membership_and_nested_lanes",
            "apps/hermes-native/src/gateway.rs#project_tree_read_uses_requested_profile_and_preview_limit",
            "apps/hermes-native/src/gateway.rs#fn load_sessions_http",
            "apps/hermes-native/src/gateway.rs#fn load_active_transcript_http",
            "apps/hermes-native/src/gateway.rs#active_transcript_refresh_reads_the_profile_scoped_latest_page_at_offset_zero",
            "apps/hermes-native/src/session_peer_sync.rs#transcript_notification_is_typed_bounded_and_contains_no_transcript_content",
            "apps/hermes-native/src/layout.rs#pub fn open_session_pane_identities",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/projects.ts#scanAndRecordRepos",
            ".staging/hermes-pinned-878305/apps/desktop/electron/git-repo-scan.ts#scanGitRepos",
            "apps/hermes-native/docs/PLAN.md#Revision 196",
            "apps/hermes-native/docs/PLAN.md#Revision 197",
            "apps/hermes-native/src/repo_discovery.rs#scan_git_repos",
            "apps/hermes-native/src/app.rs#fn show_repo_scan_settings",
            "apps/hermes-native/src/app.rs#fn start_repo_discovery_for_profile",
            "apps/hermes-native/src/app.rs#fn settle_repo_discovery",
            "apps/hermes-native/src/app.rs#repo_discovery_success_forces_project_tree_refresh_after_cached_load",
            "apps/hermes-native/src/app.rs#repo_discovery_failure_preserves_cached_project_tree_and_surfaces_error",
            "apps/hermes-native/src/app.rs#repo_discovery_result_for_previous_profile_does_not_refresh_current_tree",
            "apps/hermes-native/src/gateway.rs#pub fn scan_and_record_repos",
            "apps/hermes-native/src/gateway.rs#pub fn discover_remote_repos",
            "apps/hermes-native/src/gateway.rs#repo_scan_config_save_uses_profile_scoped_sparse_merge_patch",
            "apps/hermes-native/src/gateway.rs#local_repo_scan_is_profile_scoped_and_discards_stale_generation",
            "apps/hermes-native/src/gateway.rs#remote_repo_discovery_uses_source_params_and_rejects_missing_repo_list",
            "apps/hermes-native/src/app.rs#settings_overlay_keeps_repo_discovery_read_only_in_all_profile_scope",
        ],
        "gaps": ["full project-list metadata reconciliation beyond the separately displayed active-project pointer, cross-gateway synthetic row completeness, and dedicated all-profile nested project_sessions hydration", "live remote discovery receipts and physical Windows discovery/focus behavior", "Rust scanner throughput parity with pinned concurrency 32 has not been measured", "legacy active-list fallback and full source visibility/sleep/reconnect-settle behavior", "source-paired Windows visuals and all 499 source-acceptance receipts remain pending"],
    },
    "PAR-0013": {
        "state": "partial",
        "level": "headless_persisted_semantic_sessions_sidebar_toggle",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/store/layout.ts#toggleSidebarOpen",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/renderer/narrow-overlays.tsx#collapsibles",
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/keybinds/actions.ts#view.toggleSidebar",
            "apps/hermes-native/src/layout.rs#pub fn pane_is_open",
            "apps/hermes-native/src/layout.rs#pub fn toggle_pane",
            "apps/hermes-native/src/layout.rs#fn apply_profile_to_service_with_reopen",
            "apps/hermes-native/src/layout.rs#semantic_sidebar_and_file_browser_visibility_persist_across_modes_and_reload",
            "apps/hermes-native/src/app.rs#fn pane_controls",
            "apps/hermes-native/src/app.rs#fn handle_layout_shortcuts",
            "apps/hermes-native/src/app.rs#fn toggle_positional_side",
            "apps/hermes-native/src/app.rs#titlebar_side_controls_expose_edge_state_and_use_semantic_fallback",
            "apps/hermes-native/src/layout.rs#pub fn toggle_positional_edge",
            "apps/hermes-native/src/layout.rs#pub fn positional_edge_open",
            "apps/hermes-native/src/layout.rs#positional_edge_toggle_minimizes_and_restores_only_the_outer_group",
            "apps/hermes-native/src/layout.rs#semantic_file_visibility_restores_the_same_right_edge_group",
            "apps/hermes-native/src/layout.rs#semantic_file_cross_restore_supports_main_row_inside_column_root",
            "apps/hermes-native/src/layout.rs#nested_physical_edge_split_falls_back_to_semantic_file_toggle",
            "apps/hermes-native/src/app.rs#RegisteredUiAction",
            "apps/hermes-native/src/app.rs#canonical_pane_action_id",
            "apps/hermes-native/src/app.rs#fn show_narrow_pane_overlay",
            "apps/hermes-native/src/app.rs#narrow_pane_overlay_is_transient_below_dock_breakpoint",
            "apps/hermes-native/src/app.rs#dock_breakpoint_uses_available_content_width",
            ".staging/hermes-pinned-878305/apps/desktop/src/plugins/hermes-bots/plugin.tsx#hideOnly: true",
            ".staging/hermes-pinned-878305/apps/desktop/src/plugins/hermes-bots/data.ts#fetchRosterSnapshot",
            ".staging/hermes-pinned-878305/apps/desktop/src/plugins/hermes-bots/roster-pane.tsx#BotsPane",
            "apps/hermes-native/src/gateway.rs#parse_bot_roster",
            "apps/hermes-native/src/gateway.rs#bot_roster_read_uses_profiles_list_with_explicit_profile_scope",
            "apps/hermes-native/src/app.rs#fn load_bot_roster",
            "apps/hermes-native/src/app.rs#fn settle_bot_roster",
            "apps/hermes-native/src/app.rs#fn bots_panel",
            "apps/hermes-native/src/app.rs#bot_roster_rejects_stale_connection_results_and_keeps_last_good_rows",
            "apps/hermes-native/src/app.rs#bots_panel_renders_the_profile_scoped_roster_headlessly",
            "apps/hermes-native/src/layout.rs#bots_pane_is_hide_only_and_joins_the_sessions_tab_group",
        ],
        "gaps": ["Bots is now a static read-only native pane; conditional Bot Mode registration, boot-time re-homing after user docking, Desktop alias-route resolution, host.agents cross-connection union, bot management/search/filter/group/activity/source-health behaviors remain absent", "the 260 px source width is only represented as a native panel minimum; exact live zone sizing, arbitrary reveal aliases/open-close intents, precise mixed-zone filtering and width overrides remain unproven", "physical Windows pointer/keyboard behavior, authenticated runtime, source-paired visuals and formal acceptance remain open"],
    },
    "PAR-0014": {
        "state": "partial",
        "level": "headless_persisted_semantic_file_browser_toggle",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/store/layout.ts#toggleFileBrowserOpen",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/renderer/narrow-overlays.tsx#narrowOverlayWidth",
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/keybinds/actions.ts#view.toggleRightSidebar",
            "apps/hermes-native/src/layout.rs#pub fn pane_is_open",
            "apps/hermes-native/src/layout.rs#pub fn toggle_pane",
            "apps/hermes-native/src/layout.rs#fn apply_profile_to_service_with_reopen",
            "apps/hermes-native/src/layout.rs#semantic_sidebar_and_file_browser_visibility_persist_across_modes_and_reload",
            "apps/hermes-native/src/app.rs#fn pane_controls",
            "apps/hermes-native/src/app.rs#fn handle_layout_shortcuts",
            "apps/hermes-native/src/app.rs#fn toggle_positional_side",
            "apps/hermes-native/src/app.rs#titlebar_side_controls_expose_edge_state_and_use_semantic_fallback",
            "apps/hermes-native/src/layout.rs#pub fn toggle_positional_edge",
            "apps/hermes-native/src/layout.rs#pub fn positional_edge_open",
            "apps/hermes-native/src/layout.rs#positional_edge_toggle_minimizes_and_restores_only_the_outer_group",
            "apps/hermes-native/src/layout.rs#semantic_file_visibility_restores_the_same_right_edge_group",
            "apps/hermes-native/src/layout.rs#semantic_file_cross_restore_supports_main_row_inside_column_root",
            "apps/hermes-native/src/layout.rs#nested_physical_edge_split_falls_back_to_semantic_file_toggle",
            "apps/hermes-native/src/app.rs#RegisteredUiAction",
            "apps/hermes-native/src/app.rs#canonical_pane_action_id",
            "apps/hermes-native/src/app.rs#fn show_narrow_pane_overlay",
            "apps/hermes-native/src/app.rs#narrow_pane_overlay_is_transient_below_dock_breakpoint",
            "apps/hermes-native/src/app.rs#dock_breakpoint_uses_available_content_width",
            ".staging/hermes-pinned-878305/apps/desktop/src/plugins/hermes-bots/plugin.tsx#hideOnly: true",
            ".staging/hermes-pinned-878305/apps/desktop/src/plugins/hermes-bots/data.ts#fetchRosterSnapshot",
            ".staging/hermes-pinned-878305/apps/desktop/src/plugins/hermes-bots/roster-pane.tsx#BotsPane",
            "apps/hermes-native/src/gateway.rs#parse_bot_roster",
            "apps/hermes-native/src/gateway.rs#bot_roster_read_uses_profiles_list_with_explicit_profile_scope",
            "apps/hermes-native/src/app.rs#fn load_bot_roster",
            "apps/hermes-native/src/app.rs#fn settle_bot_roster",
            "apps/hermes-native/src/app.rs#fn bots_panel",
            "apps/hermes-native/src/app.rs#bot_roster_rejects_stale_connection_results_and_keeps_last_good_rows",
            "apps/hermes-native/src/app.rs#bots_panel_renders_the_profile_scoped_roster_headlessly",
            "apps/hermes-native/src/layout.rs#bots_pane_is_hide_only_and_joins_the_sessions_tab_group",
        ],
        "gaps": ["Bots is now a static read-only native pane; conditional Bot Mode registration, boot-time re-homing after user docking, Desktop alias-route resolution, host.agents cross-connection union, bot management/search/filter/group/activity/source-health behaviors remain absent", "the 260 px source width is only represented as a native panel minimum; exact live zone sizing, arbitrary reveal aliases/open-close intents, precise mixed-zone filtering and width overrides remain unproven", "physical Windows pointer/keyboard behavior, authenticated runtime, source-paired visuals and formal acceptance remain open"],
    },
    "PAR-0016": {
        "state": "partial",
        "level": "headless_pointer_resize_and_persisted_preferences",
        "refs": [
            ".staging/hermes-upstream-8358fc/apps/desktop/src/app/master-detail.tsx#SPLIT_MIN_LEFT_PX",
            ".staging/hermes-upstream-8358fc/apps/desktop/src/app/master-detail.tsx#onDoubleClick",
            "apps/hermes-native/docs/PLAN.md#Revision 179",
            "apps/hermes-native/src/app.rs#fn clamp_master_detail_left_width",
            "apps/hermes-native/src/app.rs#capability_catalog_master_detail_width_matches_pinned_constraints",
            "apps/hermes-native/src/app.rs#plugin_catalog_exposes_distinct_accessible_master_detail_panes",
            "apps/hermes-native/src/layout.rs#master_detail_width_override_persists_by_stable_resize_id_and_can_reset",
        ],
        "gaps": ["source behavior at narrow widths and full visual parity remain unpaired", "physical Windows pointer behavior", "live runtime and source-paired acceptance"],
    },
    "PAR-0015": {
        "state": "partial",
        "level": "headless_layout_profile_projection",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/contrib/layout-presets.ts#sidebar-left",
            "apps/hermes-native/src/layout.rs#pub fn set_profile",
            "apps/hermes-native/src/layout.rs#every_source_profile_projects_with_all_panels_open",
            "apps/hermes-native/src/layout.rs#simple_mode_hides_optional_panes_but_keeps_advanced_preferences",
        ],
        "gaps": ["no direct flip-sidebar action equivalent is exposed; orientation currently changes through layout profile selection", "physical Windows and source-paired orientation evidence remain open"],
    },
    "PAR-0017": {
        "state": "partial",
        "level": "headless_custom_layout_save_apply_delete_and_picker",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/presets.ts#saveCurrentLayoutAs",
            "apps/hermes-native/src/layout.rs#user_layout_presets_persist_geometry_and_retain_live_session_and_resting_panes",
            "apps/hermes-native/src/app.rs#user_layout_preset_picker_saves_and_lists_persisted_preset_headlessly",
        ],
        "gaps": ["physical Windows pointer/keyboard behavior and formal pinned-source acceptance remain open", "visual source pairing is not recorded; presentation polish follows functional coverage per operator direction"],
    },
    "PAR-0333": {
        "state": "partial",
        "level": "headless_keybinding_settings_capture_persistence_and_dispatch",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/settings/keybind-settings.tsx#KeybindSettings",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/keybinds.ts#bindingsFor",
            "apps/hermes-native/src/keybindings.rs#pub const NATIVE_ACTIONS",
            "apps/hermes-native/src/keybindings_ui.rs#armed_capture_records_a_supported_combo_and_clear_reset_work",
            "apps/hermes-native/src/app.rs#settings_route_switches_between_workspace_and_keybindings",
            "apps/hermes-native/src/app.rs#persisted_keybinding_override_drives_the_native_shortcut_handler",
        ],
        "gaps": ["native editor exposes only ten actions with existing handlers; remaining pinned-source actions have no native implementation", "physical Windows keyboard-layout capture and formal pinned-source acceptance remain open"],
    },
    "PAR-0334": {
        "state": "partial",
        "level": "headless_keybinding_conflict_detection_and_visible_warning",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/store/keybinds.ts#conflictsFor",
            "apps/hermes-native/src/keybindings.rs#conflicts_for",
            "apps/hermes-native/src/keybindings.rs#conflicts_are_canonical_and_ordered_by_supported_action_list",
            "apps/hermes-native/src/keybindings_ui.rs#conflict_warning_is_visible_and_reset_all_clears_overrides",
        ],
        "gaps": ["conflict precedence and collision behavior have not been compared against every pinned-source action", "physical keyboard and formal pinned-source acceptance remain open"],
    },
    "PAR-0018": {
        "state": "partial",
        "level": "headless_layout_store_reload_and_restore",
        "refs": [
            "apps/hermes-native/src/layout.rs#fn load_fallback_at",
            "apps/hermes-native/src/layout.rs#pub fn save_layout",
            "apps/hermes-native/src/layout.rs#session_panes_are_profile_scoped_and_survive_profile_close_reopen_and_restore",
            "apps/hermes-native/src/layout.rs#show_all_project_sessions_preference_survives_workspace_reload",
        ],
        "gaps": ["an explicit process-restart test for selected layout profile and split geometry is missing", "physical restart restoration and source-paired acceptance remain open"],
    },
    "PAR-0019": {
        "state": "partial",
        "level": "headless_simple_advanced_profile_projection",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/contrib/layout-presets.ts#sidebar-left",
            "apps/hermes-native/src/layout.rs#pub fn set_mode",
            "apps/hermes-native/src/layout.rs#simple_mode_hides_optional_panes_but_keeps_advanced_preferences",
        ],
        "gaps": ["source-paired state transitions and physical layout behavior are unverified", "the full source arrangement/profile workflows remain open"],
    },
    "PAR-0020": {
        "state": "partial",
        "level": "headless_source_profile_recipes",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/contrib/layout-presets.ts#BASIC_TREE",
            "apps/hermes-native/src/layout.rs#fn source_profiles",
            "apps/hermes-native/src/layout.rs#every_source_profile_projects_with_all_panels_open",
        ],
        "gaps": ["recipe-by-recipe source geometry and interaction have not been paired", "physical Windows layout acceptance remains open"],
    },
    "PAR-0021": {
        "state": "partial",
        "level": "headless_profile_scoped_session_tile_state",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/renderer/keep-alive-panes.tsx#KeepAlivePanes",
            "apps/hermes-native/src/app.rs#hidden_session_submission_receipt_updates_its_cached_pane_transcript",
            "apps/hermes-native/src/layout.rs#session_panes_are_profile_scoped_and_survive_profile_close_reopen_and_restore",
        ],
        "gaps": ["native keep-alive process/resource behavior for arbitrary hidden panes and live runtime remain unverified", "source-paired pane visibility and lifecycle acceptance remain open"],
    },
    "PAR-0022": {
        "state": "partial",
        "level": "headless_session_pane_close_and_busy_guard",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/store.ts#closeTabPane",
            "apps/hermes-native/src/layout.rs#pub fn close_session_pane",
            "apps/hermes-native/src/app.rs#active_session_pane_close_requires_confirmation_and_does_not_interrupt_hermes",
            "apps/hermes-native/src/app.rs#fn close_active_tab",
        ],
        "gaps": ["only session panes and known owned state are covered; full contributor resource-release and policy semantics remain open", "live and source-paired close behavior remains unverified"],
    },
    "PAR-0023": {
        "state": "partial",
        "level": "headless_closed_session_tile_restore",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/renderer/keep-alive-panes.tsx#KeepAlivePanes",
            "apps/hermes-native/src/layout.rs#restore_closed_session_pane_is_lifo_profile_scoped_and_focuses_restored_tile",
            "apps/hermes-native/src/app.rs#restore_closed_session_tab_action_reopens_and_focuses_the_latest_closed_tile",
        ],
        "gaps": ["closed-session descriptor restore does not prove source keep-alive hidden-pane restoration without remount", "physical runtime and source-paired lifecycle evidence remain open"],
    },
    "PAR-0026": {
        "state": "partial",
        "level": "headless_dock_tab_context_menu",
        "refs": [
            "apps/hermes-native/src/app.rs#tab_strip_with_context_menu",
            "apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs#pointer_drag_and_dock_tab_selection_use_the_live_ui_path",
        ],
        "gaps": ["only the Dock tab context menu is exercised; remaining Hermes source context menus/actions are unreviewed", "source-paired and physical interaction acceptance remain open"],
    },
    "PAR-0029": {
        "state": "partial",
        "level": "headless_dock_layout_and_active_tab_reload",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/store.ts#setActivePane",
            "apps/hermes-native/src/layout.rs#pub fn save_layout",
            "apps/hermes-native/src/layout.rs#session_panes_are_profile_scoped_and_survive_profile_close_reopen_and_restore",
        ],
        "gaps": ["full focused-workspace state, per-pane focus, crash restart and physical restart parity are not proven", "source-paired acceptance remains open"],
    },
    "PAR-0032": {
        "state": "partial",
        "level": "headless_hide_only_and_busy_session_close_guard",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/pane-shell/tree/store.ts#closeTabPane",
            "apps/hermes-native/src/layout.rs#fn base_document",
            "apps/hermes-native/src/app.rs#active_session_pane_close_requires_confirmation_and_does_not_interrupt_hermes",
        ],
        "gaps": ["source-exact handling for every pane close policy, permission class and contributor pane remains open", "live and source-paired acceptance remains open"],
    },
    "PAR-0033": {
        "state": "partial",
        "level": "headless_new_chat_session_create_and_prompt_submit",
        "refs": [
            "apps/hermes-native/src/app.rs#new_chat_send_creates_session_then_submits_draft_on_runtime_id",
            "apps/hermes-native/src/app.rs#fn begin_create_session",
            "apps/hermes-native/src/gateway.rs#for_test_session_create_and_submit",
        ],
        "gaps": ["live authenticated Gateway behavior and create/submit failure paths remain unverified", "source-paired acceptance remains open"],
    },
    "PAR-0034": {
        "state": "partial",
        "level": "headless_rendered_incremental_assistant_text",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/streaming.test.tsx#renders assistant text incrementally before completion",
            "apps/hermes-native/src/app.rs#incremental_assistant_delta_is_visible_before_terminal_message_complete",
        ],
        "gaps": ["authenticated live stream and source loading-status behavior remain unverified", "source-paired acceptance remains open"],
    },
    "PAR-0035": {
        "state": "partial",
        "level": "headless_interim_and_terminal_assistant_text_reconciliation",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/chat-messages/parts.ts#Merge the final assistant text into a message's parts",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-message-stream/gateway-event/message-stream.ts#message.interim",
            "apps/hermes-native/src/app.rs#interim_assistant_event_seals_one_bubble_and_final_text_opens_the_next",
            "apps/hermes-native/src/app.rs#cumulative_terminal_text_is_scoped_after_a_tool_delimited_interim",
            "apps/hermes-native/src/app.rs#whitespace_only_terminal_text_preserves_the_streamed_assistant_reply",
            "apps/hermes-native/src/app.rs#partial_error_terminal_text_does_not_repeat_an_interim_prefix",
            "apps/hermes-native/src/app.rs#message_stream_events_project_into_the_active_runtime_transcript",
        ],
        "gaps": ["native transcript still lacks Hermes' structured assistant text/reasoning/tool part model and reasoning disclosure", "live Gateway and source-paired acceptance remain open"],
    },
    "PAR-0003": {
        "state": "partial",
        "level": "headless_ui_and_dock_behavior",
        "refs": [
            ".staging/hermes-upstream-8358fc/apps/desktop/src/app/chat/new-session-tab.ts#frontUntouchedDraftTab()",
            ".staging/hermes-upstream-8358fc/apps/desktop/src/app/chat/new-session-tab.ts#listed: false",
            "apps/hermes-native/docs/PLAN.md#Revision 48",
            "apps/hermes-native/src/app.rs#fn create_blank_session_tab",
            "apps/hermes-native/src/app.rs#fn begin_create_session",
            "apps/hermes-native/src/app.rs#GatewayEvent::SessionCreated(result) => match result",
            "apps/hermes-native/src/app.rs#new_session_reuses_the_untouched_blank_tile_for_the_same_profile",
            "apps/hermes-native/src/app.rs#new_session_reuses_an_untouched_primary_draft_without_creating_a_session",
            "apps/hermes-native/src/app.rs#new_session_does_not_reuse_a_typed_primary_draft",
            "apps/hermes-native/src/app.rs#dynamic_session_pane_renders_its_profile_view_without_replacing_the_main_chat",
        ],
        "gaps": ["Bot Mode exact-route routing and blocked/orphan fallback are not implemented", "source-side unlisted-until-first-message behavior is not verified", "live gateway behavior", "source-paired Windows visuals and acceptance"],
    },
    "PAR-0004": {
        "state": "partial",
        "level": "headless_ui_and_dock_behavior",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 54",
            "apps/hermes-native/src/app.rs#active_session_pane_close_requires_confirmation_and_does_not_interrupt_hermes",
            "apps/hermes-native/src/app.rs#close_primary_workspace_promotes_nearest_session_pane_by_profile",
        ],
        "gaps": ["remaining close-focus and narrow-width cases", "live gateway behavior", "source-paired Windows visuals and acceptance"],
    },
    "PAR-0005": {
        "state": "partial",
        "level": "headless_ui_and_layout_behavior",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 73",
            "apps/hermes-native/src/layout.rs#restore_closed_session_pane_is_lifo_profile_scoped_and_focuses_restored_tile",
            "apps/hermes-native/src/app.rs#restore_closed_session_tab_action_reopens_and_focuses_the_latest_closed_tile",
        ],
        "gaps": ["live window lifecycle", "source-paired Windows visuals and acceptance"],
    },
    "PAR-0006": {
        "state": "partial",
        "level": "headless_pointer_and_dock_behavior",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 18",
            "apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs#pointer_drag_and_dock_tab_selection_use_the_live_ui_path",
        ],
        "gaps": ["physical Windows tab dragging", "narrow-width overflow and floating/cross-window reorder", "source-paired visuals and acceptance"],
    },
    "PAR-0001": {
        "state": "partial",
        "level": "headless_ui_and_local_http",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 138",
            "apps/hermes-native/docs/PLAN.md#Revision 139",
            "apps/hermes-native/src/app.rs#first_run_remote_setup_renders_discovered_auth_mode_and_provider",
            "apps/hermes-native/src/app.rs#first_run_setup_matches_pinned_copy_and_gates_remote_actions_by_credentials",
            "apps/hermes-native/src/remote_probe.rs#token_connection_test_checks_authenticated_http_and_websocket",
        ],
        "gaps": ["local profile selection/bootstrap", "legacy embedded/password login", "live authenticated service", "source-paired Windows behavior and visuals"],
    },
    "PAR-0025": {
        "state": "partial",
        "level": "headless_ui_and_protocol",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 134",
            "apps/hermes-native/docs/PLAN.md#Revision 135",
            "apps/hermes-native/src/app.rs#command_palette_scores_search_results_like_pinned_desktop",
            "apps/hermes-native/src/app.rs#command_palette_go_to_entries_follow_pinned_source_order",
            "apps/hermes-native/src/app.rs#command_palette_opens_searches_and_navigates_with_source_shortcuts",
        ],
        "gaps": ["full source command set", "settings and new-window context", "physical keyboard behavior", "source-paired visuals and acceptance"],
    },
    "PAR-0040": {
        "state": "partial",
        "level": "headless_accessible_code_copy_and_clipboard_payload",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/chat/shiki-highlighter.tsx#copyableCodeText",
            "apps/hermes-native/src/app.rs#markdown_fixture_renders_table_and_code_copy_action_accessibly",
            "apps/hermes-native/vendor/egui_commonmark_backend/src/elements.rs#Copy code",
        ],
        "gaps": [
            "native copy does not unwrap valid @url directive references into literal URLs like the pinned Desktop source",
            "physical Windows clipboard behavior and source-paired visuals/acceptance remain unverified",
        ],
    },
    "PAR-0254": {
        "state": "partial",
        "level": "headless_skill_toggle_http_scope_optimistic_and_rollback_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/skills.ts#setSkillEnabled",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/skills/skills-tab.tsx#applyEnabled",
            "apps/hermes-native/src/app.rs#set_skill_enabled",
            "apps/hermes-native/src/app.rs#skill_toggle_is_profile_fenced_optimistic_and_rolls_back_on_denial",
            "apps/hermes-native/src/gateway.rs#set_skill_enabled_http",
            "apps/hermes-native/src/gateway.rs#skill_toggle_uses_pinned_put_payload_and_validates_receipt",
            "apps/hermes-native/docs/PLAN.md#Revision 252",
        ],
        "gaps": [
            "native bulk skill enable/disable, disable-unused action, edit/lock/details, and install/update flows are separate and remain open",
            "rendered toggle interaction, profile-switch in-flight behavior, live Gateway, installed Windows and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0255": {
        "state": "partial",
        "level": "headless_profile_scoped_skill_detail_and_source_view",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/skills.ts#getSkillContent",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/skills/skill-detail.tsx#SkillDetail",
            "apps/hermes-native/src/gateway.rs#load_skill_content_http",
            "apps/hermes-native/src/gateway.rs#skill_content_read_uses_encoded_name_and_explicit_profile_scope",
            "apps/hermes-native/src/app.rs#open_skill_detail",
            "apps/hermes-native/src/app.rs#skill_detail_load_is_profile_fenced_and_keeps_source_path",
            "apps/hermes-native/docs/PLAN.md#Revision 265",
        ],
        "gaps": [
            "live Gateway, remote authenticated profile, installed Windows interaction and source-paired acceptance remain unverified",
            "skill edit, lock, archive and hub install or update flows are separate capabilities and remain open",
        ],
    },
    "PAR-0256": {
        "state": "partial",
        "level": "headless_public_skill_catalog_search_list_and_detail",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/catalog/catalog-data.ts#fetchCatalog",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/catalog/catalog-browser.tsx#CatalogBrowser",
            "apps/hermes-native/src/skill_catalog.rs#parse_skill_catalog",
            "apps/hermes-native/src/skill_catalog.rs#parser_normalizes_rows_ignores_malformed_entries_and_preserves_source_identity",
            "apps/hermes-native/src/skill_catalog.rs#http_reader_fails_closed_on_redirect",
            "apps/hermes-native/src/app.rs#draw_skill_hub_catalog",
            "apps/hermes-native/src/app.rs#skill_hub_detail_selection_persists_across_frames",
            "apps/hermes-native/docs/PLAN.md#Revision 266",
        ],
        "gaps": [
            "pinned installed/catalog merge, discovery shelves, facet filters, sort modes and card/list preference remain open",
            "live catalog feed, live Gateway profile, installed Windows interaction and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0257": {
        "state": "partial",
        "level": "headless_profile_scoped_skill_install_and_live_log",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/catalog/skill-catalog.tsx#install",
            ".staging/hermes-pinned-878305/apps/desktop/src/api/skills.ts#installSkillFromHub",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/hub-actions.ts#runHubAction",
            ".staging/hermes-pinned-878305/hermes_cli/web_routers/skills.py#install_skill_hub",
            "apps/hermes-native/src/skill_catalog.rs#skill_catalog_install_identifier",
            "apps/hermes-native/src/gateway.rs#install_skill_hub_http",
            "apps/hermes-native/src/gateway.rs#skill_hub_install_posts_profile_scoped_identifier_and_returns_action_name",
            ".staging/hermes-pinned-878305/apps/desktop/src/api/skills.ts#scanSkillHub",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/hub-actions.ts#notifyHubActionFailed",
            ".staging/hermes-pinned-878305/hermes_cli/web_routers/skills.py#scan_skill_hub",
            "apps/hermes-native/src/skill_catalog.rs#parse_skill_hub_install_block",
            "apps/hermes-native/src/skill_catalog.rs#validate_skill_hub_scan_result",
            "apps/hermes-native/src/skill_catalog.rs#install_block_parser_matches_current_and_legacy_security_gate_messages",
            "apps/hermes-native/src/skill_catalog.rs#scan_result_requires_matching_identifier_bounded_fields_and_known_policy",
            "apps/hermes-native/src/gateway.rs#load_skill_hub_scan_http",
            "apps/hermes-native/src/gateway.rs#skill_hub_scan_reads_profile_scoped_findings_without_mutating_install_state",
            "apps/hermes-native/src/app.rs#request_skill_hub_install",
            "apps/hermes-native/src/app.rs#request_skill_hub_scan",
            "apps/hermes-native/src/app.rs#settle_skill_hub_action_status",
            "apps/hermes-native/src/app.rs#skill_hub_detail_selection_persists_across_frames",
            "apps/hermes-native/src/app.rs#skill_hub_blocked_install_exposes_view_scan_without_policy_override",
            "apps/hermes-native/docs/PLAN.md#Revision 268",
        ],
        "gaps": [
            "live Gateway, Windows interaction, install process cancellation, and source-paired acceptance remain unverified",
            "the retry uses the pinned ordinary install action and never overrides a Hermes security refusal",
        ],
    },
    "PAR-0258": {
        "state": "partial",
        "level": "headless_profile_scoped_skill_update_and_live_log",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/catalog/update-skills-button.tsx#UpdateSkillsButton",
            ".staging/hermes-pinned-878305/apps/desktop/src/api/skills.ts#updateSkillsFromHub",
            ".staging/hermes-pinned-878305/hermes_cli/web_routers/skills.py#update_skills_hub",
            "apps/hermes-native/src/gateway.rs#update_skill_hub_skills_http",
            "apps/hermes-native/src/gateway.rs#skill_hub_update_posts_once_to_explicit_profile_and_validates_receipt",
            "apps/hermes-native/src/gateway.rs#skill_hub_update_rejects_mismatched_or_unconfirmed_receipts",
            "apps/hermes-native/src/app.rs#start_skill_hub_update",
            "apps/hermes-native/src/app.rs#settle_skill_hub_update_status",
            "apps/hermes-native/src/app.rs#installed_skills_page_exposes_update_action_and_action_log",
            "apps/hermes-native/src/app.rs#skill_hub_update_status_is_profile_fenced_and_settles_on_success",
            "apps/hermes-native/docs/PLAN.md#Revision 269",
        ],
        "gaps": [
            "live Gateway update, packaged Windows interaction, cancellation, uncertain-start reconciliation, and source-paired/formal acceptance remain unverified",
            "pinned per-skill update checks and individual update workflows remain separate from the batch Update installed action",
        ],
    },
    "PAR-0253": {
        "state": "partial",
        "level": "headless_skill_hub_category_facet_and_search_test",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/catalog/catalog-browser.tsx#CatalogFilterBar",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/catalog/catalog-filter-bar.tsx#onSelect",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/catalog/catalog-query.ts#catalogCategories",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/catalog/catalog-query.ts#filterCatalog",
            "apps/hermes-native/src/app.rs#draw_skill_hub_catalog",
            "apps/hermes-native/src/app.rs#skill_hub_detail_selection_persists_across_frames",
            "apps/hermes-native/docs/PLAN.md#Revision 270",
        ],
        "gaps": [
            "live Gateway, packaged Windows interaction, and source-paired/formal acceptance remain unverified",
            "catalog/source/tag facets, installed-row merging, discovery shelves, sort modes, and card/list preference remain open",
        ],
    },
    "PAR-0259": {
        "state": "partial",
        "level": "headless_toolsets_inventory_included_tools_and_visibility_test",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/toolsets.ts#getToolsets",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/toolsets/toolsets-tab.tsx#item",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/toolsets/toolsets-data.ts#filteredToolsets",
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/desktop-toolsets.ts#isDesktopToolsetVisible",
            "apps/hermes-native/src/gateway.rs#load_capabilities",
            "apps/hermes-native/src/app.rs#desktop_toolset_visible",
            "apps/hermes-native/src/app.rs#toolsets_catalog_renders_included_tools_and_hides_desktop_internal_rows",
            "apps/hermes-native/docs/PLAN.md#Revision 271",
        ],
        "gaps": [
            "live Gateway, packaged Windows interaction, and source-paired/formal acceptance remain unverified",
            "source themes/featured sections, tool-call ranking/counts, and toolset detail/configuration views remain open",
        ],
    },
    "PAR-0260": {
        "state": "partial",
        "level": "headless_toolsets_master_toggle_sequential_rollback_and_disconnect_test",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/toolsets/toolsets-tab.tsx#bulkApply",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/toolsets/toolsets-tab.tsx#CatalogSwitch",
            ".staging/hermes-pinned-878305/apps/desktop/src/api/toolsets.ts#setToolsetEnabled",
            "apps/hermes-native/src/gateway.rs#request_toolset_enabled",
            "apps/hermes-native/src/app.rs#set_toolsets_enabled_bulk",
            "apps/hermes-native/src/app.rs#advance_toolset_bulk_update",
            "apps/hermes-native/src/app.rs#toolsets_master_toggle_uses_full_visible_inventory_sequentially_and_stops_on_denial",
            "apps/hermes-native/docs/PLAN.md#Revision 272",
        ],
        "gaps": [
            "live Gateway, packaged Windows interaction, and source-paired/formal acceptance remain unverified",
            "Toolsets themes/featured ranking, tool-call usage counts, and toolset detail/configuration views remain open",
        ],
    },
    "PAR-0261": {
        "state": "partial",
        "level": "headless_toolsets_prerequisite_readiness_indicator_test",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/toolsets/toolset-detail.tsx#DetailHeader",
            ".staging/hermes-pinned-878305/apps/desktop/src/types/hermes.ts#ToolsetInfo",
            "apps/hermes-native/src/gateway.rs#ToolsetInfo",
            "apps/hermes-native/src/app.rs#capabilities_content",
            "apps/hermes-native/src/app.rs#toolsets_catalog_renders_included_tools_and_hides_desktop_internal_rows",
            "apps/hermes-native/docs/PLAN.md#Revision 273",
        ],
        "gaps": [
            "the current API exposes only a configured boolean; individual required environment keys/providers are not shown",
            "live Gateway, packaged Windows interaction, and source-paired/formal acceptance remain unverified",
        ],
    },
    "PAR-0269": {
        "state": "not_implemented",
        "level": "source_audited_native_desktop_plugin_host_gap",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/plugins/plugin-page-access.tsx#PluginPageAccess",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/plugin-preview-grants.ts#GRANTS_KEY",
            "apps/hermes-native/src/gateway.rs#pub struct AgentPluginInfo",
            "apps/hermes-native/src/app.rs#agent_plugin_is_manageable",
        ],
        "gaps": [
            "pinned permission grants are Electron-local preview-script grants, while Hermes Native has no Desktop renderer-plugin host, capability metadata, origin-grant store, or revoke UI",
            "a faithful native implementation depends on a separate native Desktop-plugin host and app-local permission persistence design",
        ],
    },
    "PAR-0262": {
        "state": "partial",
        "level": "headless_mcp_inventory_profile_scoped_http_and_accessible_ui_test",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/mcp.ts#listMcpServers",
            ".staging/hermes-pinned-878305/apps/desktop/src/types/hermes.ts#McpServerSummary",
            "apps/hermes-native/src/gateway.rs#load_mcp_servers_http",
            "apps/hermes-native/src/gateway.rs#mcp_server_inventory_uses_profile_scoped_ticketed_get",
            "apps/hermes-native/src/app.rs#CapabilitiesTab::Connectors",
            "apps/hermes-native/src/app.rs#connectors_directory_shows_connected_state_and_account_details",
            "apps/hermes-native/docs/PLAN.md#Revision 253",
        ],
        "gaps": [
            "MCP server add, edit, test, enable/disable, remove, catalog and integration install workflows remain open",
            "live Gateway, remote authenticated profile, rendered installed UI, Windows packaging and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0265": {
        "state": "partial",
        "level": "headless_mcp_server_toggle_http_profile_and_rollback_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/mcp.ts#setMcpServerEnabled",
            "apps/hermes-native/src/gateway.rs#set_mcp_server_enabled_http",
            "apps/hermes-native/src/gateway.rs#mcp_server_toggle_uses_pinned_profile_scoped_put_and_receipt",
            "apps/hermes-native/src/app.rs#set_mcp_server_enabled",
            "apps/hermes-native/src/app.rs#mcp_server_toggle_is_profile_fenced_optimistic_and_rolls_back_on_denial",
            "apps/hermes-native/docs/PLAN.md#Revision 254",
        ],
        "gaps": [
            "pinned success receipt only returns ok; live Gateway state reconciliation has not been exercised",
            "remote authenticated profile, disconnect-under-mutation, rendered UI, Windows and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0264": {
        "state": "partial",
        "level": "headless_mcp_connection_test_profile_fenced_result_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/mcp.ts#testMcpServer",
            "apps/hermes-native/src/gateway.rs#test_mcp_server_http",
            "apps/hermes-native/src/gateway.rs#mcp_server_test_uses_pinned_profile_scoped_post_and_returns_tools",
            "apps/hermes-native/src/app.rs#test_mcp_server",
            "apps/hermes-native/src/app.rs#mcp_server_test_shows_result_and_ignores_stale_profile_response",
            "apps/hermes-native/docs/PLAN.md#Revision 255",
        ],
        "gaps": [
            "automatic probe and five-minute cache behavior from the pinned source is not implemented; current slice is the manual connection test",
            "live/remote Gateway, concurrent profile changes, rendered installed UI, Windows packaging and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0263": {
        "state": "partial",
        "level": "headless_mcp_stdio_http_add_rpc_secret_and_accessibility_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/connectors/add-dialog.tsx#controller.addServerEntry",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/connectors/add-server-draft.ts#entryOfDraft",
            ".staging/hermes-pinned-878305/apps/shared/src/gateway-contract.generated.ts#McpServersAddParams",
            "apps/hermes-native/src/gateway.rs#mcp_server_add_params",
            "apps/hermes-native/src/gateway.rs#mcp_server_add_uses_profile_scoped_rpc_and_keeps_bearer_out_of_result",
            "apps/hermes-native/src/app.rs#mcp_server_add_sends_config_and_secret_and_fences_result",
            "apps/hermes-native/src/app.rs#mcp_server_add_form_renders_transport_controls_and_submit_accessibly",
            "apps/hermes-native/docs/PLAN.md#Revision 256",
        ],
        "gaps": [
            "the pinned paste/import parser and environment passthrough rows are not implemented",
            "live profile-secret storage, OAuth authorization, server startup, Windows interaction and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0266": {
        "state": "partial",
        "level": "headless_confirmed_mcp_remove_rpc_profile_and_stale_receipt_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/shared/src/gateway-contract.generated.ts#McpServersRemoveResult",
            "apps/hermes-native/src/gateway.rs#parse_mcp_server_remove_result",
            "apps/hermes-native/src/gateway.rs#mcp_server_remove_uses_profile_scoped_rpc_and_requires_removed_receipt",
            "apps/hermes-native/src/app.rs#request_mcp_server_removal",
            "apps/hermes-native/src/app.rs#mcp_server_removal_requires_current_confirmation_and_fences_receipt",
            "apps/hermes-native/docs/PLAN.md#Revision 257",
        ],
        "gaps": [
            "live Gateway, profile credential cleanup, actual MCP worker shutdown and restart behavior are not verified",
            "installed Windows interaction and pinned-source paired acceptance remain open",
        ],
    },
    "PAR-0267": {
        "state": "partial",
        "level": "headless_profile_scoped_mcp_catalog_rest_and_accessible_state_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/mcp.ts#getMcpCatalog",
            ".staging/hermes-pinned-878305/hermes_cli/web_routers/mcp.py#list_mcp_catalog",
            "apps/hermes-native/src/gateway.rs#parse_mcp_catalog_result",
            "apps/hermes-native/src/gateway.rs#mcp_catalog_uses_desktop_rest_contract_and_validates_install_metadata",
            "apps/hermes-native/src/app.rs#load_mcp_catalog",
            "apps/hermes-native/src/app.rs#mcp_catalog_load_is_profile_fenced_and_renders_accessible_states",
            "apps/hermes-native/docs/PLAN.md#Revision 259",
        ],
        "gaps": [
            "live Gateway, remote profile, installed Windows UI and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0268": {
        "state": "partial",
        "level": "headless_profile_scoped_secret_safe_install_and_background_action_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/mcp.ts#installMcpCatalogEntry",
            ".staging/hermes-pinned-878305/hermes_cli/web_routers/mcp.py#install_mcp_catalog_entry",
            "apps/hermes-native/src/gateway.rs#mcp_catalog_install_posts_profile_scoped_env_and_returns_background_action",
            "apps/hermes-native/src/gateway.rs#mcp_catalog_install_http_failure_does_not_echo_credentials",
            "apps/hermes-native/src/app.rs#mcp_catalog_install_masks_required_values_and_tracks_background_action",
            "apps/hermes-native/src/app.rs#mcp_catalog_install_disconnect_marks_sync_outcome_uncertain",
            "apps/hermes-native/docs/PLAN.md#Revision 260",
        ],
        "gaps": [
            "live Gateway secret persistence, authenticated remote behavior, installed Windows behavior and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0252": {
        "state": "partial",
        "level": "native_installed_skill_search_fields_tested",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/skills/skills-data.ts#filteredSkills",
            "apps/hermes-native/src/app.rs#filter_installed_skills",
            "apps/hermes-native/src/app.rs#installed_skills_search_matches_name_category_and_description",
            "apps/hermes-native/docs/PLAN.md#Revision 250",
            "apps/hermes-native/docs/PLAN.md#Revision 251",
        ],
        "gaps": [
            "native tie-breaking uses lowercase ordinal name ordering, which may differ from pinned localeCompare for locale-specific names",
            "skill search UI/hint behavior and rendered results lack dedicated interaction coverage",
            "profile changes, live Gateway, Windows interaction, and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0251": {
        "state": "partial",
        "level": "native_installed_skill_rows_and_search_filter_test",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/index.tsx#CapabilitiesView",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/skills/skills-data.ts#useSkillsQuery",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/capabilities/skills/skills-data.ts#filteredSkills",
            "apps/hermes-native/src/app.rs#CapabilitiesTab::Skills",
            "apps/hermes-native/src/app.rs#filter_installed_skills",
            "apps/hermes-native/src/app.rs#installed_skills_search_matches_name_category_and_description",
            "apps/hermes-native/src/gateway.rs#SkillInfo",
            "apps/hermes-native/docs/PLAN.md#Revision 249",
        ],
        "gaps": [
            "native installed-skill list has no dedicated headless rendering test and does not match pinned usage ordering/category metadata presentation",
            "skill toggle, details/source, hub/install/update lifecycle, load-error UI, and profile-switch cache behavior are not established by this row",
            "live Gateway, installed Windows interaction, and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0041": {
        "state": "partial",
        "level": "headless_generation_scoped_accessible_tool_cards",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback.tsx#ToolRunHeader",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback.tsx#ToolRun",
            "apps/hermes-native/src/app.rs#tool_start_and_complete_render_generation_scoped_transcript_cards",
            "apps/hermes-native/src/app.rs#upsert_tool_row",
            "apps/hermes-native/docs/PLAN.md#Revision 248",
        ],
        "gaps": [
            "native renders each tool call as an individual transcript card instead of grouping consecutive calls into the pinned summary/disclosure run",
            "pinned live ticker, persisted run disclosure, approval-aware collapse, and specialized tool fallback rendering are not matched",
            "live Gateway, installed Windows behavior, and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0042": {
        "state": "partial",
        "level": "headless_advanced_mode_accessible_tool_payload_disclosure",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback.tsx#ToolPayloadDisclosure",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback.tsx#technicalTrace",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback-model/format.ts#MAX_TOOL_RENDER_CHARS",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback.test.ts#indents object payloads and persisted JSON strings",
            "apps/hermes-native/src/app.rs#tool_payload_trace",
            "apps/hermes-native/src/app.rs#pretty_tool_payload_value",
            "apps/hermes-native/src/app.rs#tool_payload_disclosure_expands_arguments_and_result_accessibly",
            "apps/hermes-native/src/app.rs#technical_tool_payload_is_hidden_in_simple_interface_mode",
            "apps/hermes-native/src/app.rs#tool_payload_trace_formats_json_and_bounds_large_results",
            "apps/hermes-native/docs/PLAN.md#Revision 263",
        ],
        "gaps": [
            "native advanced mode provides the disclosure but does not yet expose the pinned independent persisted technical/product tool-view preference",
            "live Gateway payload behavior, installed Windows interaction, and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0043": {
        "state": "partial",
        "level": "headless_session_profile_scoped_tool_progress_rows_and_cleanup",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-message-stream/gateway-event/tools.ts#handleToolProgressEvent",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/tool-progress.ts#setToolProgress",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-message-stream/gateway-event/tools-progress.test.ts#shows tool.progress on its running row and drops it when the call completes",
            "apps/hermes-native/src/app.rs#update_tool_call_progress",
            "apps/hermes-native/src/app.rs#tool_progress_text",
            "apps/hermes-native/src/app.rs#tool_progress_annotates_running_rows_and_retires_on_completion_or_turn_end",
            "apps/hermes-native/src/app.rs#background_tool_progress_isolated_by_profile_for_reused_runtime_and_tool_ids",
            "apps/hermes-native/src/app.rs#tool_progress_fans_out_to_batched_rows_and_bounds_display_text",
            "apps/hermes-native/docs/PLAN.md#Revision 262",
        ],
        "gaps": [
            "native evidence is headless; real MCP notifications, interrupted-turn parity, installed Windows rendering, and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0044": {
        "state": "partial",
        "level": "headless_tool_image_preview_and_enlarge_action",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback-model/index.ts#toolImageUrl",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/chat/zoomable-image.tsx#ZoomableImage",
            "apps/hermes-native/src/app.rs#tool_image_reference",
            "apps/hermes-native/src/app.rs#tool_image_open_action_is_accessible_and_sets_the_lightbox_target",
            "apps/hermes-native/src/app.rs#tool_image_reference_follows_result_then_arguments_and_filters_non_images",
            "apps/hermes-native/src/files.rs#show_image_thumbnail",
            "apps/hermes-native/src/gateway.rs#load_remote_artifact_image_preview",
            "apps/hermes-native/docs/PLAN.md#Revision 264",
        ],
        "gaps": [
            "remote enlarge currently uses the bounded 512x384 preview instead of a full-resolution source image; data:image URLs are not decoded into native previews",
            "zoom and pan gestures, image load retry, live Gateway behavior, installed Windows interaction, and source-paired acceptance remain open",
        ],
    },
    "PAR-0045": {
        "state": "partial",
        "level": "headless_rendered_inline_tool_diff_with_explicit_outcome_status",
        "refs": [
            ".staging/hermes-pinned-878305/tui_gateway/contracts/events.py#ToolCompletePayload",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback.tsx#FileDiffPanel",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/fallback-model/index.ts#toolStatus",
            "apps/hermes-native/src/app.rs#tool_completion_status",
            "apps/hermes-native/src/app.rs#tool_inline_diff",
            "apps/hermes-native/src/app.rs#render_tool_inline_diff",
            "apps/hermes-native/src/app.rs#inline_tool_diff_is_visible_alongside_result_and_has_outcome_status",
            "apps/hermes-native/src/app.rs#tool_completion_status_uses_explicit_failure_and_success_precedence",
            "apps/hermes-native/src/app.rs#tool_inline_diff_accepts_legacy_field_and_bounds_large_payloads",
            "apps/hermes-native/docs/PLAN.md#Revision 313",
        ],
        "gaps": [
            "headless UI and event fixtures do not prove live Hermes Gateway result envelopes or packaged Windows rendering",
            "formal pinned-source acceptance remains pending; diffs are bounded to 80,000 characters and 1,500 rendered lines",
        ],
    },
    "PAR-0053": {
        "state": "partial",
        "level": "headless_deep_rewind_consent_and_target_revalidation",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-prompt-actions/rewind.ts#truncateSubmitParams",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-prompt-actions/rewind.ts#confirmIfDeepCut",
            "apps/hermes-native/src/app.rs#editing_a_persisted_message_requires_deep_cut_confirmation_and_preserves_composer",
            "apps/hermes-native/src/app.rs#edited_message_revalidates_target_and_submits_confirmed_gateway_truncation",
            "apps/hermes-native/src/app.rs#prior_message_edit_refuses_when_later_turn_count_changed_after_confirmation",
            "apps/hermes-native/docs/PLAN.md#Revision 318",
        ],
        "gaps": [
            "headless consent and stale-target tests do not prove authenticated live Gateway or packaged Windows behavior",
            "formal pinned-source acceptance remains pending",
        ],
    },
    "PAR-0054": {
        "state": "partial",
        "level": "headless_transcript_bottom_follow_and_reader_offset_preservation",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/list.tsx#useStickToBottom",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/list.tsx#thread.runStart",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/scroll-to-bottom-button.tsx#requestScrollToBottom",
            "apps/hermes-native/src/app.rs#transcript_scroll_area",
            "apps/hermes-native/src/app.rs#transcript_scroll_follows_growth_until_reader_leaves_and_returns_to_bottom",
            "apps/hermes-native/docs/PLAN.md#Revision 319",
        ],
        "gaps": [
            "the headless egui state regression does not exercise physical Windows wheel input or a live streaming Gateway",
            "packaged Windows behavior and formal pinned-source acceptance remain pending",
        ],
    },
    "PAR-0055": {
        "state": "partial",
        "level": "headless_session_scoped_transcript_read_position",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/store/thread-scroll.ts#threadScrollStateFromMetrics",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/list.tsx#planThreadScrollRestore",
            "apps/hermes-native/src/app.rs#TranscriptScrollPosition",
            "apps/hermes-native/src/app.rs#transcript_read_position_is_session_owned_and_restores_from_bottom",
            "apps/hermes-native/docs/PLAN.md#Revision 320",
        ],
        "gaps": [
            "native evidence covers in-process per-session read-offset state but not durable cross-restart storage, history-prepend reanchoring, or cancellation by physical Windows input",
            "live Gateway, packaged Windows, and formal pinned-source acceptance remain open",
        ],
    },
    "PAR-0056": {
        "state": "partial",
        "level": "headless_indexed_prompt_timeline_and_around_row_window",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/timeline.tsx#ThreadTimeline",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/timeline-data.ts#deriveTimelineEntries",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/history-window.ts#fetchHistoryWindow",
            ".staging/hermes-pinned-878305/hermes_cli/web_routers/sessions.py#get_session_timeline",
            ".staging/hermes-pinned-878305/hermes_cli/web_routers/sessions.py#get_session_messages_around",
            ".staging/hermes-pinned-878305/hermes_state_timeline.py#get_session_messages_around",
            "apps/hermes-native/src/app.rs#transcript_prompt_timeline_entries",
            "apps/hermes-native/src/app.rs#rendered_prompt_timeline_targets_the_existing_transcript_row",
            "apps/hermes-native/src/app.rs#rendered_prompt_history_index_selects_a_durable_unloaded_row",
            "apps/hermes-native/src/app.rs#around_history_result_replaces_only_its_active_owner_and_scrolls_to_anchor",
            "apps/hermes-native/src/app.rs#around_history_result_from_stale_connection_cannot_replace_active_chat",
            "apps/hermes-native/src/gateway.rs#prompt_history_window_remote_read_uses_around_route_and_validates_anchor",
            "apps/hermes-native/src/gateway.rs#prompt_history_window_rejects_wrong_anchor_and_inconsistent_bounds",
            "apps/hermes-native/src/gateway.rs#prompt_timeline_remote_read_uses_profile_scope_and_metadata_only_route",
            "apps/hermes-native/src/gateway.rs#prompt_timeline_page_rejects_scope_and_pagination_mismatches",
            "apps/hermes-native/docs/PLAN.md#Revision 323",
        ],
        "gaps": [
            "automatic index TTL/invalidation and active marker tracking during manual scrolling remain open; an explicit accessible refresh action is available",
            "physical Windows input, packaged behavior, authenticated live Hermes Gateway and formal pinned-source acceptance remain open",
        ],
    },
    "PAR-0059": {
        "state": "partial",
        "level": "headless_background_complete_transcript_routing_and_owner_fence",
        "refs": [
            ".staging/hermes-pinned-878305/tui_gateway/contracts/events.py#SideAgentCompletePayload",
            ".staging/hermes-pinned-878305/tui_gateway/contracts/events.py#background.complete",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-message-stream/gateway-event/status.ts#background.complete",
            "apps/hermes-native/src/app.rs#append_background_completion_notice",
            "apps/hermes-native/src/app.rs#background_completion_notice_is_persistent_bounded_and_task_idempotent",
            "apps/hermes-native/src/app.rs#background_completion_is_routed_by_session_and_profile_owner",
            "apps/hermes-native/docs/PLAN.md#Revision 314",
        ],
        "gaps": [
            "cron completion history and persistent subagent completion are not part of the pinned Desktop background.complete transcript contract",
            "live Gateway, packaged Windows, and formal pinned-source acceptance remain open",
        ],
    },
    "PAR-0046": {
        "state": "partial",
        "level": "headless_ui_gateway_config_and_stream_reducer",
        "refs": [
            "apps/hermes-native/src/reasoning.rs#three_character_frames_preserve_safe_answer_and_extract_reasoning",
            "apps/hermes-native/src/app.rs#reasoning_stream_is_separate_and_partial_tags_never_enter_answer",
            "apps/hermes-native/src/app.rs#disabled_reasoning_preference_hides_the_disclosure_and_text",
            "apps/hermes-native/src/gateway.rs#display_reasoning_config_read_uses_profile_scoped_source_api",
        ],
        "gaps": ["native UI control to change display.show_reasoning is not implemented", "live Gateway reasoning delivery and source-paired Windows acceptance remain open"],
    },
    "PAR-0047": {
        "state": "partial",
        "level": "headless_accessible_ui",
        "refs": [
            "apps/hermes-native/src/app.rs#simple_mode_keeps_reasoning_in_a_collapsed_accessible_disclosure",
        ],
        "gaps": ["source-paired and packaged Windows acceptance remains open"],
    },
    "PAR-0036": {
        "state": "partial",
        "level": "headless_full_app_commonmark_markdown_blocks_and_code_fences",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/markdown-text.tsx#MarkdownTextContent",
            "apps/hermes-native/src/app.rs#render_transcript_markdown",
            "apps/hermes-native/src/app.rs#markdown_fixture_renders_table_and_code_copy_action_accessibly",
        ],
        "gaps": ["pinned Streamdown preprocessing, media/directive/math integrations and packaged Windows behavior are not fully ported or verified"],
    },
    "PAR-0037": {
        "state": "partial",
        "level": "headless_fenced_code_render_and_copy_with_native_highlighter_enabled",
        "refs": [
            "apps/hermes-native/Cargo.toml#egui_commonmark",
            "apps/hermes-native/src/app.rs#render_transcript_markdown",
            "apps/hermes-native/src/app.rs#markdown_fixture_renders_table_and_code_copy_action_accessibly",
        ],
        "gaps": ["syntax token colors, language aliases, theme behavior and packaged Windows rendering lack focused assertions and source-paired acceptance"],
    },
    "PAR-0038": {
        "state": "partial",
        "level": "headless_rendered_gfm_table_and_list_content",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/markdown-table.tsx#ResizableMarkdownTable",
            "apps/hermes-native/src/app.rs#markdown_fixture_renders_table_and_code_copy_action_accessibly",
        ],
        "gaps": ["resizable table behavior, narrow-width rendering, and live/source-paired acceptance remain open"],
    },
    "PAR-0039": {
        "state": "partial",
        "level": "headless_local_project_preview_and_safe_external_link_dispatch",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/markdown-text.tsx#function MarkdownLink",
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/external-link.tsx#openLink",
            "apps/hermes-native/src/app.rs#safe_transcript_link_url",
            "apps/hermes-native/src/app.rs#safe_transcript_preview_path",
            "apps/hermes-native/src/app.rs#markdown_file_preview_resolution_stays_inside_local_project",
            "apps/hermes-native/src/app.rs#markdown_local_file_link_opens_dock_preview_only_for_matching_local_session",
            "apps/hermes-native/src/app.rs#markdown_web_links_open_only_after_safe_url_validation",
            "apps/hermes-native/src/app.rs#markdown_unsafe_links_never_emit_host_navigation_commands",
            "apps/hermes-native/src/app.rs#markdown_link_policy_normalizes_web_domains_and_rejects_active_schemes",
        ],
        "gaps": ["remote filesystem previews need session-scoped read endpoints and remote content caching; audio/video players, pinned media/directive preprocessing, and richer Markdown behavior remain open", "packaged Windows, live Gateway and source-paired acceptance remain open"],
    },
    "PAR-0049": {
        "state": "partial",
        "level": "headless_owner_scoped_stale_runtime_interrupt_recovery",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 291",
            "apps/hermes-native/src/gateway.rs#session_interrupt_resumes_stored_session_and_retries_once_when_runtime_is_stale",
            "apps/hermes-native/src/gateway.rs#stale_interrupt_resume_failure_preserves_original_error_and_does_not_retry",
            "apps/hermes-native/src/app.rs#recovered_interrupt_rebinds_only_the_matching_session_view",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-prompt-actions/index.sleep-wake.test.tsx#resumes the stored session and retries once when session.interrupt reports",
        ],
        "gaps": [
            "live Gateway and packaged Windows Stop behavior remain unverified",
            "source-paired acceptance is pending",
        ],
    },
    "PAR-0050": {
        "state": "partial",
        "level": "headless_truncated_reply_continue_action_and_submit_fallback",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 290",
            "apps/hermes-native/src/app.rs#structured_model_and_provider_errors_render_actionable_copy_with_collapsed_details",
            "apps/hermes-native/src/app.rs#truncated_reply_continue_submits_through_the_existing_session_when_idle",
            "apps/hermes-native/src/app.rs#truncated_reply_continue_preserves_existing_draft_when_submit_is_unavailable",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/assistant-message.tsx#continueCutOffReply",
        ],
        "gaps": [
            "live Gateway turn continuation and packaged Windows interaction remain unverified",
            "source-paired acceptance is pending",
        ],
    },
    "PAR-0051": {
        "state": "partial",
        "level": "headless_owner_scoped_prior_message_edit_stale_runtime_recovery_failed_turn_and_history_row_resolution",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 304",
            "apps/hermes-native/docs/PLAN.md#Revision 307",
            "apps/hermes-native/src/app.rs#edit_without_durable_row_waits_for_history_then_uses_resolved_truncation_id",
            "apps/hermes-native/src/app.rs#unresolved_edit_target_falls_back_to_plain_submit_without_truncation",
            "apps/hermes-native/src/app.rs#stale_history_resolution_owner_never_submits_an_edit",
            "apps/hermes-native/src/gateway.rs#durable_edit_row_resolution_requires_exact_unique_persisted_user_match",
            "apps/hermes-native/src/gateway.rs#durable_edit_row_resolution_uses_last_duplicate_only_for_proven_newest_turn",
            "apps/hermes-native/src/gateway.rs#edit_row_history_lookup_uses_oldest_ordered_profile_scoped_pages",
            "apps/hermes-native/docs/PLAN.md#Revision 305",
            "apps/hermes-native/src/app.rs#editing_a_persisted_message_requires_deep_cut_confirmation_and_preserves_composer",
            "apps/hermes-native/src/app.rs#edited_message_revalidates_target_and_submits_confirmed_gateway_truncation",
            "apps/hermes-native/src/app.rs#prior_message_edit_refuses_when_later_turn_count_changed_after_confirmation",
            "apps/hermes-native/src/app.rs#editing_during_a_running_turn_interrupts_before_truncating_submit",
            "apps/hermes-native/src/app.rs#busy_edit_interrupts_then_retries_once_and_preserves_history_on_second_refusal",
            "apps/hermes-native/src/app.rs#message_edit_uploads_new_image_without_touching_composer_and_sends_gateway_reference",
            "apps/hermes-native/src/app.rs#message_edit_waits_for_new_file_upload_before_sending_truncation",
            "apps/hermes-native/src/app.rs#stale_edit_runtime_rebind_updates_only_the_matching_connection_owner",
            "apps/hermes-native/src/gateway.rs#rpc_preserves_gateway_busy_code_for_bounded_prompt_recovery",
            "apps/hermes-native/src/gateway.rs#edit_prompt_resumes_stale_runtime_and_retries_exact_truncation_once",
            "apps/hermes-native/src/gateway.rs#edit_prompt_does_not_retry_when_resume_returns_the_wrong_owner",
            "apps/hermes-native/src/app.rs#unknown_edit_interrupt_preserves_history_and_never_sends_the_truncating_prompt",
            "apps/hermes-native/src/app.rs#refused_prior_message_edit_preserves_composer_and_restores_edit_text",
            "apps/hermes-native/src/app.rs#failed_turn_without_row_id_exposes_edit_dialog_without_truncation_warning",
            "apps/hermes-native/src/app.rs#failed_turn_without_durable_row_uses_plain_submit_and_replaces_only_after_admission",
            "apps/hermes-native/src/app.rs#refused_failed_turn_edit_keeps_original_error_and_draft_for_retry",
            "apps/hermes-native/src/gateway.rs#prompt_edit_uses_stable_row_and_explicit_deep_cut_confirmation",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0087.json#EV-0087",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0088.json#EV-0088",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0089.json#EV-0089",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0090.json#EV-0090",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0092.json#EV-0092",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0093.json#EV-0093",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0094.json#EV-0094",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-prompt-actions/rewind.ts#planConfirmedEdit",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-prompt-actions/rewind.ts#planEdit",
            ".staging/hermes-pinned-878305/hermes_cli/web_routers/sessions.py#get_session_messages",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/user-edit-composer.tsx#handleDrop",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/user-message.tsx#editable={!readOnly}",
            ".staging/hermes-pinned-878305/tui_gateway/methods_prompt.py#_truncate_history_for_submit",
        ],
        "gaps": [
            "live Gateway stale-runtime resume, admission/refusal, uncertainty and race behavior remain unverified",
            "live Gateway behavior, packaged Windows picker/drop interaction, concurrent offset-pagination stability, and formal pinned-source acceptance remain unverified",
        ],
    },
    "PAR-0052": {
        "state": "partial",
        "level": "headless_regenerate_previous_answer_with_durable_row_rebinding",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 303",
            "apps/hermes-native/src/app.rs#regeneration_target_uses_the_nearest_unique_durable_user_row",
            "apps/hermes-native/src/app.rs#regenerate_action_opens_a_read_only_prompt_and_confirms_deep_cut",
            "apps/hermes-native/src/app.rs#regeneration_rebinds_survivors_after_admission_and_preserves_composer",
            "apps/hermes-native/src/gateway.rs#truncating_prompt_receipt_returns_rebound_and_new_user_row_ids",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0091.json#EV-0091",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-prompt-actions/rewind.ts#planReload",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-prompt-actions/rewind.ts#planConfirmedReload",
            ".staging/hermes-pinned-878305/tui_gateway/methods_prompt.py#_truncate_history_for_submit",
        ],
        "gaps": [
            "live Gateway admission/refusal and post-rewrite transcript reconciliation remain unverified",
            "regeneration is intentionally unavailable when the prior user prompt lacks a unique positive durable row ID",
            "packaged Windows interaction and fixed-source acceptance remain open",
        ],
    },
    "PAR-0057": {
        "state": "partial",
        "level": "headless_around_history_and_index_predecessor_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/history-window.ts#fetchHistoryWindow",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/history-window.test.tsx#latest request wins even when the bridge ignores cancellation",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/history-window.test.tsx#shows the older page on its own when a turn longer than the page limit separates it from the anchor",
            "apps/hermes-native/src/app.rs#older_transcript_history_exposes_an_accessible_control",
            "apps/hermes-native/src/app.rs#older_transcript_page_prepends_and_deduplicates_boundary_overlap",
            "apps/hermes-native/src/app.rs#show_earlier_uses_the_prompt_index_predecessor_as_around_row_anchor",
            "apps/hermes-native/src/app.rs#show_earlier_pages_the_timeline_index_until_it_reaches_the_window_anchor",
            "apps/hermes-native/src/app.rs#older_around_window_replaces_visible_page_when_a_long_turn_leaves_a_gap",
            "apps/hermes-native/src/app.rs#older_prompt_lookup_uses_the_nearest_predecessor_and_checks_contiguity",
            "apps/hermes-native/src/gateway.rs#older_transcript_page_uses_profile_scope_and_validates_latest_offset",
            "apps/hermes-native/src/gateway.rs#older_transcript_page_rejects_a_different_profile_receipt",
            "apps/hermes-native/docs/PLAN.md#Revision 325",
        ],
        "gaps": [
            "explicit cancellation/latest-request replacement for an in-flight older-history reveal is not implemented; request/session/profile/connection receipts fence stale responses",
            "physical scroll-anchor preservation during contiguous older-window prepend is unverified",
            "authenticated live Gateway behavior, packaged Windows interaction, and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0058": {
        "state": "partial",
        "level": "headless_loaded_transcript_search_keyboard_and_match_highlight",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/find-in-page-scope.test.ts#wraps every (case-insensitive) match in a <mark> and counts them",
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/find-in-page-scope.test.ts#advances the active mark on findNext and walks back on findPrevious",
            "apps/hermes-native/src/app.rs#transcript_find_opens_searches_cycles_highlights_and_closes_from_keyboard",
            "apps/hermes-native/src/app.rs#fn transcript_search_matches",
            "apps/hermes-native/docs/PLAN.md#Implement the pinned Desktop `view.findInPage` interaction",
        ],
        "gaps": [
            "native search is limited to loaded transcript rows and highlights matching message cards rather than each rendered substring",
            "other rendered chat panes/views are not searched",
            "live Gateway, installed Windows keyboard behavior, and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0062": {
        "state": "partial",
        "level": "headless_gateway_payload_and_composer_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-prompt-actions/index.ts#uploadComposerAttachment",
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/attachment-upload-policy.ts#attachmentPathNeedsUpload",
            "apps/hermes-native/src/gateway.rs#composer_attachment_rpc_payload",
            "apps/hermes-native/src/gateway.rs#composer_image_upload_uses_pinned_gateway_bytes_contract",
            "apps/hermes-native/src/gateway.rs#composer_file_upload_uses_pinned_gateway_data_url_contract",
            "apps/hermes-native/src/app.rs#compose_prompt_with_attachments",
            "apps/hermes-native/src/app.rs#prompt_composer_places_gateway_attachment_refs_before_user_text",
            "apps/hermes-native/src/gateway.rs#prompt_submit_includes_source_image_attachment_metadata",
            "apps/hermes-native/src/app.rs#image_only_prompt_uses_source_default_text_and_image_ref_stays_in_attachment_metadata",
            "apps/hermes-native/src/app.rs#composer_drop_accepts_multiple_paths_only_when_released_over_input",
            "apps/hermes-native/src/app.rs#fn composer_file_drop_paths",
            "apps/hermes-native/src/app.rs#fn composer_drop_path_list_is_complete",
        ],
        "gaps": ["live Hermes Gateway upload and refusal behavior", "pinned local path versus byte-upload policy", "native OS local-path drop is only a basic route into the existing upload path; previews, clipboard paste, pathless drops, and retry after session recovery remain open", "interrupted image draft restore after app restart", "Windows native picker and side-by-side source acceptance"],
    },
    "PAR-0116": {
        "state": "partial",
        "level": "headless_native_os_file_drop_routing",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/composer/hooks/use-composer-drop.ts#handleDrop",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/composer/hooks/use-composer-drop.ts#handleInputDrop",
            "apps/hermes-native/src/app.rs#accept_composer_file_drop",
            "apps/hermes-native/src/app.rs#composer_drop_accepts_multiple_paths_only_when_released_over_input",
            "apps/hermes-native/src/app.rs#fn composer_file_drop_paths",
            "apps/hermes-native/src/app.rs#fn composer_drop_path_list_is_complete",
        ],
        "gaps": ["only local paths exposed by egui are routed; any drop containing a pathless item is rejected as a whole with a fallback to Attach file", "custom Hermes path references, clipboard images, Gateway outcome matrix, live Windows interaction, and source-paired acceptance remain open"],
    },
    "PAR-0063": {
        "state": "partial",
        "level": "headless_ui_and_local_http",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/composer/prepared-image-recovery.tsx#listPreparedImageDrafts",
            "apps/hermes-native/src/prepared_image_drafts.rs#prepared_image_drafts_round_trip_and_retire_atomically",
            "apps/hermes-native/src/app.rs#prepared_image_restore_is_scoped_and_does_not_send",
            "apps/hermes-native/src/app.rs#restore_prepared_image_draft",
        ],
        "gaps": ["live Gateway validity of persisted attachment references after process restart", "Windows packaged app restart/picker behavior", "source-paired runtime and manual acceptance"],
    },
    "PAR-0064": {
        "state": "partial",
        "level": "headless_ui_and_local_http",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-message-stream/agent-init-error.test.tsx#renders an agent-init failure as a visible in-transcript error and keeps the optimistic first message",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/assistant-message.test.tsx#names the provider and keeps Retry for a rate limit",
            "apps/hermes-native/src/app.rs#failed_turn_events_keep_user_row_merge_in_either_order_and_release_busy_state",
            "apps/hermes-native/src/app.rs#retry_failed_turn_reuses_submission_id_and_keeps_one_user_row",
            "apps/hermes-native/src/app.rs#retry_failed_turn_blocks_nonretryable_errors_and_preserves_occupied_composer",
        ],
        "gaps": ["code-specific alternate actions such as edit, choose model, sign in, or start a new session are not implemented", "full error-surface action taxonomy and scheduled retry behavior are not implemented", "live Gateway failure variants and source-paired runtime acceptance remain pending"],
    },
    "PAR-0065": {
        "state": "partial",
        "level": "headless_ui",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/timeline-timestamp.tsx#TimelineTimestamp",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/tool/tool-group.test.tsx#tool lifecycle timestamps",
            "apps/hermes-native/src/app.rs#transcript_timestamps_read_upstream_metadata_and_format_local_ranges",
            "apps/hermes-native/src/app.rs#fn format_timeline_tooltip",
        ],
        "gaps": ["tool-run lifecycle timestamp mapping from structured nested parts", "live Gateway behavior", "source-paired Windows acceptance"],
    },
    "PAR-0067": {
        "state": "partial",
        "level": "headless_gateway_scoped_keyboard_slash_completion_and_draft_insertion",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/composer/index.tsx#event.key === 'ArrowDown'",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/composer/index.tsx#event.key === 'ArrowUp'",
            "apps/hermes-native/src/app.rs#slash_completion_requires_a_leading_single_line_command",
            "apps/hermes-native/src/app.rs#slash_suggestion_filter_uses_pinned_desktop_surface_and_skill_rows",
            "apps/hermes-native/src/app.rs#slash_suggestion_inserts_a_draft_without_submitting_it",
            "apps/hermes-native/src/app.rs#slash_suggestions_support_keyboard_selection_without_sending",
            "apps/hermes-native/src/gateway.rs#slash_completion_actor_routes_catalog_and_typed_queries_to_the_scoped_gateway",
            "apps/hermes-native/docs/PLAN.md#Revision 244",
            "apps/hermes-native/docs/PLAN.md#Revision 293",
        ],
        "gaps": ["directive chips and mid-message trigger behavior are not implemented", "suggestion cache invalidation and special local /skin and session commands are not implemented", "live Gateway behavior, installed Windows interaction, source-paired acceptance, and visual pairing remain unverified"],
    },
    "PAR-0511": {
        "state": "partial",
        "level": "headless_replay_gap_active_and_open_tile_recovery",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-message-stream/replay-gap.test.tsx#requests an authoritative resume for the affected active route without losing owner scope or its draft",
            "apps/hermes-native/src/app.rs#replay_gap_resumes_active_session_and_preserves_transcript_and_draft",
            "apps/hermes-native/src/app.rs#replay_gap_resumes_only_the_exact_open_session_pane_owner",
        ],
        "gaps": ["automatic WebSocket reconnect is not implemented; recovery currently handles replay-gap events after a Gateway is connected", "live Hermes replay-gap behavior and authoritative resume after a production disconnect are unverified", "ambiguous multi-profile runtime ownership is fail-closed but lacks a targeted regression", "installed Windows, source-paired visual evidence, and formal source acceptance remain pending"],
    },
    "PAR-0512": {
        "state": "partial",
        "level": "headless_canonical_event_sequence_dedup",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/api/canonical-reconnect-replay-owner.test.ts#canonical reconnect replays the retained window to the epoch owner",
            "apps/hermes-native/src/gateway_event_dedup.rs#suppresses_duplicate_sequence_only_for_same_owner_epoch_and_connection",
            "apps/hermes-native/src/gateway_event_dedup.rs#unsequenced_legacy_events_are_never_suppressed",
            "apps/hermes-native/src/gateway_event_dedup.rs#retention_is_bounded_and_old_keys_can_be_relearned",
            "apps/hermes-native/src/app.rs#handle_server_event_from_connection",
        ],
        "gaps": ["deduplication retains only the latest 8,192 canonical identities, so much older retransmissions can repeat", "live Gateway reconnect/replay has not been verified", "Windows runtime and formal pinned-source acceptance remain pending"],
    },
    "PAR-0075": {
        "state": "implemented",
        "level": "headless_protocol_and_persistence_test",
        "refs": [
            "docs/BACKLOG.md#PAR-0075",
            "apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs#pointer_drag_reorders_pinned_session_rows_and_preserves_unloaded_pins",
        ],
        "gaps": ["authenticated live profile", "source-paired Windows drag and visual acceptance"],
    },
    "PAR-0076": {
        "state": "partial",
        "level": "headless_protocol_and_persistence_test",
        "refs": [
            "apps/hermes-native/docs/BACKLOG.md#PAR-0076",
            "apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs#pointer_drag_reorders_recent_rows_within_date_groups",
        ],
        "gaps": ["physical Windows pointer behavior", "source-paired visuals", "authenticated live acceptance"],
    },
    "PAR-0078": {
        "state": "partial",
        "level": "headless_protocol_and_ui_test",
        "refs": [
            "docs/BACKLOG.md#PAR-0078",
            "apps/hermes-native/docs/ARCHITECTURE.md#PAR-0078",
            "apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs#session_appearance_menu_sets_and_clears_a_persisted_color",
        ],
        "gaps": ["source-paired Windows visual behavior", "authenticated live project-color behavior"],
    },
    "PAR-0282": {
        "state": "implemented",
        "level": "headless_ui_and_mapping_test",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 153",
            "apps/hermes-native/src/app.rs#messaging_status_mapping_matches_pinned_platform_status_logic",
            "apps/hermes-native/src/app.rs#messaging_inventory_renders_each_status_group_and_filters_the_channel_list",
        ],
        "gaps": ["live authenticated gateway behavior", "full Messaging source flow", "source-paired Windows acceptance"],
    },
    "PAR-0300": {
        "state": "implemented",
        "level": "local_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 161",
            "apps/hermes-native/src/gateway.rs#remote_cron_jobs_read_is_profile_scoped_authenticated_and_bounded",
            "apps/hermes-native/src/app.rs#scheduled_jobs_route_renders_profile_owned_list_and_selected_details",
        ],
        "gaps": ["live authenticated gateway", "source-paired behavior and visuals"],
    },
    "PAR-0301": {
        "state": "partial",
        "level": "profile_scoped_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 169",
            "apps/hermes-native/src/gateway.rs#cron_crud_and_delivery_targets_use_profile_scoped_authenticated_contracts",
            "apps/hermes-native/src/gateway.rs#model_options_and_cron_blueprints_use_pinned_profile_scoped_api_contracts",
            "apps/hermes-native/src/app.rs#scheduled_job_blueprint_form_renders_profile_owned_typed_slots",
            "apps/hermes-native/src/app.rs#scheduled_job_blueprint_defaults_and_validation_match_source_contract",
            "apps/hermes-native/src/app.rs#scheduled_job_editor_keeps_saved_model_selectable_when_catalog_omits_it",
            "apps/hermes-native/src/app.rs#scheduled_jobs_route_renders_profile_owned_list_and_selected_details",
        ],
        "gaps": ["source-paired Windows behavior and visuals", "live authenticated Hermes acceptance", "localized source labels and blueprint error mapping"],
    },
    "PAR-0302": {
        "state": "partial",
        "level": "profile_scoped_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 169",
            "apps/hermes-native/src/gateway.rs#cron_crud_and_delivery_targets_use_profile_scoped_authenticated_contracts",
            "apps/hermes-native/src/app.rs#scheduled_job_blueprint_defaults_and_validation_match_source_contract",
            "apps/hermes-native/src/app.rs#scheduled_jobs_route_renders_profile_owned_list_and_selected_details",
        ],
        "gaps": ["acceptance matrix asks for timezone editing, while pinned Desktop editor exposes no timezone field", "live authenticated Hermes acceptance", "source-paired Windows behavior and visuals"],
    },
    "PAR-0303": {
        "state": "implemented",
        "level": "local_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 162",
            "apps/hermes-native/src/gateway.rs#cron_pause_resume_actions_use_profiled_empty_posts_and_validate_receipts",
            "apps/hermes-native/src/app.rs#scheduled_jobs_route_renders_profile_owned_list_and_selected_details",
        ],
        "gaps": ["live authenticated gateway", "source-paired behavior and visuals"],
    },
    "PAR-0304": {
        "state": "implemented",
        "level": "local_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 162",
            "apps/hermes-native/src/gateway.rs#cron_pause_resume_actions_use_profiled_empty_posts_and_validate_receipts",
            "apps/hermes-native/src/app.rs#scheduled_jobs_route_renders_profile_owned_list_and_selected_details",
        ],
        "gaps": ["live authenticated gateway", "source-paired behavior and visuals"],
    },
    "PAR-0305": {
        "state": "partial",
        "level": "local_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 164",
            "apps/hermes-native/src/gateway.rs#cron_trigger_isolated_from_http_worker_and_cancel_settles_independently",
            "apps/hermes-native/src/app.rs#scheduled_jobs_route_renders_profile_owned_list_and_selected_details",
        ],
        "gaps": ["live authenticated gateway", "source-paired behavior and visuals"],
    },
    "PAR-0306": {
        "state": "partial",
        "level": "profile_scoped_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 169",
            "apps/hermes-native/src/gateway.rs#cron_crud_and_delivery_targets_use_profile_scoped_authenticated_contracts",
            "apps/hermes-native/src/app.rs#scheduled_job_delete_requires_confirmation_before_transport",
        ],
        "gaps": ["live authenticated Hermes acceptance", "source-paired confirmation dialog and behavior visuals"],
    },
    "PAR-0307": {
        "state": "implemented",
        "level": "local_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 169",
            "apps/hermes-native/src/gateway.rs#cron_delivery_target_read_accepts_oauth_bearer_and_validates_target_rows",
            "apps/hermes-native/src/app.rs#scheduled_jobs_route_renders_profile_owned_list_and_selected_details",
        ],
        "gaps": ["live authenticated Hermes acceptance", "source-paired Windows labels and visuals"],
    },
    "PAR-0308": {
        "state": "partial",
        "level": "local_http_and_headless_ui",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 163",
            "apps/hermes-native/src/gateway.rs#cron_run_history_read_is_profile_scoped_bounded_and_rejects_duplicates",
            "apps/hermes-native/src/app.rs#scheduled_jobs_route_renders_profile_owned_list_and_selected_details",
        ],
        "gaps": ["run-row navigation into its session", "live authenticated gateway", "source-paired behavior and visuals"],
    },
    "PAR-0510": {
        "state": "partial",
        "level": "headless_active_reselect_and_loopback_inflight_resume_recovery",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/contrib/wiring.tsx#openStoredSession",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-route-resume.ts#dispatchRouteResume",
            "apps/hermes-native/src/app.rs#reselecting_the_active_sidebar_session_forces_a_profile_scoped_reattach_without_submit",
            "apps/hermes-native/src/app.rs#explicit_session_resume_skips_rows_with_archive_or_delete_in_flight",
            "apps/hermes-native/src/app.rs#session_resume_errors_are_correlated_and_render_on_the_owning_tile",
            "apps/hermes-native/src/gateway.rs#fn resume_session",
            "apps/hermes-native/src/gateway.rs#session_resume_streams_inflight_events_and_recovers_after_refusal",
            "apps/hermes-native/docs/PLAN.md#Revision 228",
        ],
        "gaps": [
            "real Hermes gateway/in-flight-turn reattachment and production reconnect behavior not exercised; loopback WebSocket verifies stream forwarding and retry after refusal",
            "native row identity preserves profile but not the source connection-plus-profile owner route",
            "Windows installed-app and source-paired visual evidence pending",
            "formal source acceptance remains pending",
        ],
    },
    "PAR-0214": {
        "state": "partial",
        "level": "headless_profile_scoped_model_and_reasoning_controls",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 279",
            "apps/hermes-native/src/app.rs#composer_exposes_model_and_reasoning_controls_for_supported_session_model",
            "apps/hermes-native/src/gateway.rs#chat_model_options_read_uses_profile_session_and_provider_capabilities",
            "apps/hermes-native/src/gateway.rs#chat_model_mutation_receipt_rejects_stale_or_wrong_owner_receipts",
        ],
        "gaps": [
            "headless UI fixture verifies control visibility but does not click a model/reasoning menu item",
            "live authenticated Gateway behavior, packaged Windows interaction and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0215": {
        "state": "partial",
        "level": "headless_profile_scoped_model_switch_with_fresh_revision_fencing",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 279",
            "apps/hermes-native/src/app.rs#composer_exposes_model_and_reasoning_controls_for_supported_session_model",
            "apps/hermes-native/src/gateway.rs#chat_model_pick_uses_fresh_profile_scoped_revision_and_generation",
            "apps/hermes-native/src/gateway.rs#guarded_chat_model_receipt_requires_confirmation_token",
            "apps/hermes-native/src/gateway.rs#chat_policy_error_receipts_and_unknown_statuses_fail_closed",
        ],
        "gaps": [
            "headless UI fixture verifies control visibility but does not click a model menu item",
            "custom model/provider management is tracked separately in PAR-0216/PAR-0217",
            "live authenticated Gateway behavior, packaged Windows interaction and source-paired acceptance remain unverified",
        ],
    },
    "PAR-0216": {
        "state": "partial",
        "level": "headless_custom_model_entry_validation_storage_and_catalog_visibility",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 280",
            "apps/hermes-native/src/app.rs#custom_model_candidate_trims_rejects_whitespace_and_suppresses_catalog_duplicates",
            "apps/hermes-native/src/app.rs#composer_exposes_model_and_reasoning_controls_for_supported_session_model",
            "apps/hermes-native/src/layout.rs#custom_model_ids_are_provider_scoped_persisted_and_validated",
            "apps/hermes-native/src/gateway.rs#chat_model_pick_uses_fresh_profile_scoped_revision_and_generation",
        ],
        "gaps": [
            "headless UI test opens the editor and renders candidates but does not activate a candidate through a successful UI-to-Gateway receipt",
            "live authenticated Gateway, packaged Windows behavior and source-paired acceptance remain open",
        ],
    },
    "PAR-0217": {
        "state": "partial",
        "level": "headless_custom_provider_slug_options_and_mutation_contract",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 280",
            "apps/hermes-native/src/app.rs#composer_exposes_model_and_reasoning_controls_for_supported_session_model",
            "apps/hermes-native/src/gateway.rs#chat_model_options_read_uses_profile_session_and_provider_capabilities",
            "apps/hermes-native/src/gateway.rs#chat_model_pick_uses_fresh_profile_scoped_revision_and_generation",
        ],
        "gaps": [
            "headless fixtures verify exact custom provider slug parsing and mutation but do not activate the provider picker row through egui",
            "custom provider endpoint configuration, live authenticated Gateway behavior, packaged Windows interaction and source-paired acceptance remain open",
        ],
    },
    "PAR-0218": {
        "state": "partial",
        "level": "headless_rendered_catalog_retry_preserves_chat_draft_and_attachment",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 281",
            "apps/hermes-native/src/app.rs#Retry model list",
            "apps/hermes-native/src/app.rs#retrying_chat_model_catalog_preserves_draft_and_attachments_on_failure",
            "apps/hermes-native/src/app.rs#retry_chat_model_options",
        ],
        "gaps": [
            "pinned Desktop reports model catalog errors without a manual retry button, so the native retry is an added recovery behavior",
            "headless test covers retry while the Gateway is unavailable; live Gateway recovery, packaged Windows behavior and source-paired acceptance remain open",
        ],
    },
    "PAR-0219": {
        "state": "partial",
        "level": "headless_structured_model_provider_error_cards",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 282",
            "apps/hermes-native/src/app.rs#structured_provider_error_presentation",
            "apps/hermes-native/src/app.rs#structured_model_and_provider_errors_render_actionable_copy_with_collapsed_details",
            ".staging/hermes-pinned-878305/apps/desktop/src/lib/error-surface-copy.ts#errorCardText",
            ".staging/hermes-pinned-878305/apps/desktop/src/components/assistant-ui/thread/assistant-message.tsx#const ErrorCard",
        ],
        "gaps": [
            "headless rendering covers model-not-found and provider-policy copy, provider/model identity, partial response and structured error details; cause-specific recovery actions and localization are not implemented",
            "live error-surface Gateway events, packaged Windows behavior and source-paired acceptance remain open",
        ],
    },
    "PAR-0220": {
        "state": "partial",
        "level": "headless_profile_scoped_fallback_route_configuration",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 283",
            "apps/hermes-native/docs/PLAN.md#Revision 285",
            "apps/hermes-native/src/app.rs#model_fallback_settings_render_ordered_routes_and_keep_blank_drafts_local",
            "apps/hermes-native/src/app.rs#fallback_model_options_results_are_fenced_by_request_and_profile",
            "apps/hermes-native/src/app.rs#fallback_route_edits_preserve_route_specific_config_and_legacy_values",
            "apps/hermes-native/src/gateway.rs#fallback_providers_local_read_and_save_are_profile_scoped_and_preserve_route_keys",
            "apps/hermes-native/src/gateway.rs#fallback_providers_remote_save_uses_profile_scope_and_session_token",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/settings/fallback-models-field.tsx#FallbackModelsField",
        ],
        "gaps": [
            "full UI save/read resynchronization is not yet covered by a rendered interaction regression",
            "Hermes owns fallback execution; live default-model failure followed by a configured fallback route, packaged Windows behavior and source-paired acceptance remain open",
        ],
    },
    "PAR-0240": {
        "state": "partial",
        "level": "headless_saved_remote_auth_fail_closed",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 295",
            "apps/hermes-native/src/app.rs#start_saved_gateway_connection",
            "apps/hermes-native/src/app.rs#saved_remote_connection_without_credentials_does_not_fall_back_to_local",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0085.json#EV-0085",
        ],
        "gaps": [
            "headless evidence covers the missing-credential failure path only; successful remote login and endpoint authentication require a live Gateway",
            "saved-connection selection UI and complete owner/result fencing across connection switches remain open",
            "packaged Windows behavior and source-paired acceptance remain pending",
        ],
    },
    "PAR-0060": {
        "state": "partial",
        "level": "headless_restart_profile_and_cache_eviction_tests",
        "refs": [
            "apps/hermes-native/src/composer_drafts.rs#pub fn load",
            "apps/hermes-native/src/composer_drafts.rs#pub fn save",
            "apps/hermes-native/src/composer_drafts.rs#composer_drafts_round_trip_by_full_owner_and_clear_atomically",
            "apps/hermes-native/src/composer_drafts.rs#composer_draft_store_caps_at_fifty_and_keeps_the_current_write",
            "apps/hermes-native/src/composer_drafts.rs#malformed_or_oversized_draft_fails_closed_without_returning_text",
            "apps/hermes-native/src/app.rs#composer_draft_survives_app_restart_and_isolated_by_profile",
            "apps/hermes-native/src/app.rs#composer_draft_cache_protects_nonempty_views_beyond_thirty_two_sessions",
            "apps/hermes-native/src/app.rs#new_chat_composer_draft_restores_after_restart_and_clear_removes_storage",
            "apps/hermes-native/docs/PLAN.md#Revision 310",
        ],
        "gaps": [
            "packaged Windows persistence behavior and multi-process/window races remain unverified",
            "attachments remain memory-only in this slice",
            "live Gateway is not required for local storage behavior but Gateway-connected end-to-end acceptance remains separate",
        ],
    },
    "PAR-0061": {
        "state": "partial",
        "level": "rendered_model_pick_and_owner_scoped_receipt_preserves_draft_and_attachments",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/store/composer.ts#$composerDraft",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/composer.ts#setComposerDraft",
            ".staging/hermes-pinned-878305/apps/desktop/src/store/session.ts#setCurrentModel",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-model-controls.ts#paintSelection",
            "apps/hermes-native/src/app.rs#mutate_chat_policy",
            "apps/hermes-native/src/app.rs#fn update_gateway_discovery",
            "apps/hermes-native/src/app.rs#model_picker_preserves_unsent_composer_state_on_success_refusal_and_confirmation",
            "apps/hermes-native/docs/PLAN.md#Revision 311",
        ],
        "gaps": [
            "live Gateway model-switch success, denial and confirmation behavior remain unverified",
            "packaged Windows interaction and source-paired acceptance remain pending",
        ],
    },
    "PAR-0048": {
        "state": "partial",
        "level": "headless_owner_scoped_session_usage_toolbar",
        "refs": [
            "apps/hermes-native/docs/PLAN.md#Revision 296",
            "apps/hermes-native/src/app.rs#session_usage_accepts_top_level_gateway_counters_and_preserves_partial_updates",
            "apps/hermes-native/src/app.rs#session_usage_is_owner_scoped_merged_and_rendered_in_chat_toolbar",
            ".staging/hermes-pinned-878305/apps/shared/src/usage-tick.ts#usageFromTick",
            ".staging/hermes-pinned-878305/gateway/session_usage.py#usage_frame",
            "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0086.json#EV-0086",
        ],
        "gaps": [
            "live Gateway event timing and offline presentation have not been verified",
            "packaged Windows behavior and pinned-source paired acceptance remain open",
        ],
    },
}

FIELDS = [
    "test_id",
    "capability_id",
    "source_snapshot",
    "priority",
    "phase",
    "scenario",
    "negative_path",
    "implementation_state",
    "evidence_level",
    "evidence_refs_json",
    "known_gaps_json",
    "live_runtime_state",
    "visual_pair_state",
    "source_pair_state",
    "reviewed_on",
]


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8-sig", newline="") as handle:
        return list(csv.DictReader(handle))


def check_refs(refs: list[str]) -> None:
    for ref in refs:
        rel_path, separator, locator = ref.partition("#")
        if not separator or not locator:
            raise ValueError(f"evidence ref must use path#exact_locator syntax: {ref}")
        path = ROOT / rel_path
        if not path.is_file():
            raise ValueError(f"evidence file not found: {rel_path}")
        content = path.read_text(encoding="utf-8", errors="replace")
        if locator not in content:
            raise ValueError(f"evidence locator not found in {rel_path}: {locator}")


def write_atomic(path: Path, rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
    temp = Path(temp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=FIELDS, lineterminator="\n")
            writer.writeheader()
            writer.writerows(rows)
            handle.flush()
            os.fsync(handle.fileno())
        check = read_rows(temp)
        if len(check) != len(rows) or any(set(row) != set(FIELDS) for row in check):
            raise ValueError("temporary progress ledger failed schema/count validation")
        ids = [row["test_id"] for row in check]
        if len(ids) != len(set(ids)):
            raise ValueError("temporary progress ledger has duplicate test IDs")
        os.replace(temp, path)
    except BaseException:
        temp.unlink(missing_ok=True)
        raise


def build(output: Path) -> dict[str, object]:
    source_rows = read_rows(SOURCE)
    prior_rows = {row["test_id"]: row for row in read_rows(output)} if output.is_file() else {}
    pinned = [row for row in source_rows if row.get("source_snapshot") == PIN]
    if len(source_rows) != 565 or len(pinned) != 499:
        raise ValueError(f"fixed source scope changed: total={len(source_rows)}, pinned={len(pinned)}")
    if any(row.get("result") != "pending" for row in pinned):
        raise ValueError("fixed acceptance rows changed; review source-paired status before generating overlay")

    ids = {row["test_id"] for row in pinned}
    unknown_seed_ids = set(EVIDENCE) - ids
    if unknown_seed_ids:
        raise ValueError(f"evidence seed IDs are not in fixed scope: {sorted(unknown_seed_ids)}")
    for entry in EVIDENCE.values():
        refs = entry["refs"]
        assert isinstance(refs, list)
        check_refs(refs)

    today = date.today().isoformat()
    rows: list[dict[str, str]] = []
    for source in pinned:
        evidence = EVIDENCE.get(source["test_id"])
        prior = prior_rows.get(source["test_id"])
        evidence_unchanged = bool(
            evidence
            and prior
            and prior.get("implementation_state") == str(evidence["state"])
            and prior.get("evidence_level") == str(evidence["level"])
            and prior.get("evidence_refs_json")
            == json.dumps(evidence["refs"], ensure_ascii=False)
            and prior.get("known_gaps_json")
            == json.dumps(evidence["gaps"], ensure_ascii=False)
        )
        rows.append(
            {
                "test_id": source["test_id"],
                "capability_id": source["capability_id"],
                "source_snapshot": source["source_snapshot"],
                "priority": source["priority"],
                "phase": source["phase"],
                "scenario": source["scenario"],
                "negative_path": source["negative_path"],
                "implementation_state": str(evidence["state"]) if evidence else "unknown",
                "evidence_level": str(evidence["level"]) if evidence else "not_reviewed",
                "evidence_refs_json": json.dumps(evidence["refs"], ensure_ascii=False) if evidence else "[]",
                "known_gaps_json": json.dumps(evidence["gaps"], ensure_ascii=False) if evidence else "[]",
                "live_runtime_state": "not_verified" if not source.get("backend_receipt") else "evidence_present_review_pending",
                "visual_pair_state": (
                    "paired_review_pending"
                    if source.get("source_capture") and source.get("target_capture")
                    else "not_paired"
                ),
                "source_pair_state": source["result"],
                "reviewed_on": (
                    prior["reviewed_on"]
                    if evidence_unchanged and prior is not None
                    else today if evidence else ""
                ),
            }
        )

    states = Counter(row["implementation_state"] for row in rows)
    if sum(states.values()) != 499 or states["unknown"] != 499 - len(EVIDENCE):
        raise ValueError(f"unexpected implementation-state counts: {dict(states)}")
    write_atomic(output, rows)
    return {
        "status": "ok",
        "output": str(output),
        "scope": len(rows),
        "explicitly_reviewed": len(EVIDENCE),
        "implementation_states": dict(sorted(states.items())),
        "source_acceptance": dict(Counter(row["source_pair_state"] for row in rows)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    print(json.dumps(build(args.output), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
