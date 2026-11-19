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
    "PAR-0057": {
        "state": "partial",
        "level": "headless_older_transcript_page_and_gateway_scope_tests",
        "refs": [
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/history-window.ts#fetchHistoryWindow",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/history-window.test.tsx#latest request wins even when the bridge ignores cancellation",
            ".staging/hermes-pinned-878305/apps/desktop/src/app/chat/history-window.test.tsx#shows the older page on its own when a turn longer than the page limit separates it from the anchor",
            "apps/hermes-native/src/app.rs#older_transcript_history_exposes_an_accessible_control",
            "apps/hermes-native/src/app.rs#older_transcript_page_prepends_and_deduplicates_boundary_overlap",
            "apps/hermes-native/src/gateway.rs#older_transcript_page_uses_profile_scope_and_validates_latest_offset",
            "apps/hermes-native/src/gateway.rs#older_transcript_page_rejects_a_different_profile_receipt",
            "apps/hermes-native/docs/PLAN.md#Revision 247",
        ],
        "gaps": [
            "native pagination fetches offset-based latest pages and lacks the pinned around-row anchor/predecessor-index flow used to reveal a selected older prompt",
            "native merge has no around-window contiguity check to show a separated older page on its own",
            "live Gateway behavior, installed Windows interaction, stale/cancelled request UI coverage, and source-paired acceptance remain unverified",
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
        ],
        "gaps": ["live Hermes Gateway upload and refusal behavior", "pinned local path versus byte-upload policy", "attachment preview, paste/drop and retry after session recovery", "interrupted image draft restore after app restart", "Windows native picker and side-by-side source acceptance"],
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
        "level": "headless_gateway_scoped_slash_completion_and_draft_insertion",
        "refs": [
            "apps/hermes-native/src/app.rs#slash_completion_requires_a_leading_single_line_command",
            "apps/hermes-native/src/app.rs#slash_suggestion_filter_uses_pinned_desktop_surface_and_skill_rows",
            "apps/hermes-native/src/app.rs#slash_suggestion_inserts_a_draft_without_submitting_it",
            "apps/hermes-native/src/gateway.rs#slash_completion_actor_routes_catalog_and_typed_queries_to_the_scoped_gateway",
            "apps/hermes-native/docs/PLAN.md#Revision 244",
        ],
        "gaps": ["keyboard arrow navigation, directive chips, and mid-message trigger behavior are not implemented", "suggestion cache invalidation and special local /skin and session commands are not implemented", "live Gateway behavior, installed Windows interaction, source-paired acceptance, and visual pairing remain unverified"],
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
                "reviewed_on": today if evidence else "",
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
