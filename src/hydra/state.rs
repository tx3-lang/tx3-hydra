use std::collections::HashMap;

use tracing::info;

use super::model::{Event, EventMeta, HeadStatus, TxID, Utxo};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    pub seq: u64,
    pub timestamp: String,
}

#[derive(Default)]
pub struct HeadState {
    pub progress: Progress,
    pub utxos: HashMap<TxID, Utxo>,
    pub head_status: HeadStatus,
}

impl HeadState {
    pub fn apply(&mut self, event: &Event, meta: &EventMeta) {
        match event {
            Event::Greetings {
                head_status,
                snapshot,
            } => {
                self.utxos = snapshot.clone();
                self.head_status = head_status.clone();
                info!(utxos = self.utxos.len(), "Greetings event");
            }
            Event::HeadIsOpen { snapshot } => {
                self.utxos = snapshot.clone();
                self.head_status = HeadStatus::Open;
                info!(utxos = self.utxos.len(), "Head is open");
            }
            Event::SnapshotConfirmed { snapshot } => {
                self.utxos = snapshot.clone().full_utxo();
                info!(utxos = self.utxos.len(), "Snapshot updated");
            }
            Event::CommitApproved { utxo_to_commit } => {
                self.utxos.extend(utxo_to_commit.clone());
                info!(utxos = utxo_to_commit.len(), "Commit approved");
            }
            Event::CommitRecovered {
                recovered_utxo,
                recovered_tx_id,
            } => {
                for tx_id in recovered_utxo.keys() {
                    self.utxos.remove(tx_id);
                }
                info!(%recovered_tx_id, utxos = recovered_utxo.len(), "Commit recovered");
            }
            Event::CommitRecorded {
                pending_deposit,
                utxo_to_commit,
            } => info!(%pending_deposit, utxos = utxo_to_commit.len(), "Commit recorded"),
            Event::DepositActivated { deposit_tx_id } => {
                info!(%deposit_tx_id, "Deposit activated")
            }
            Event::DepositExpired { deposit_tx_id } => info!(%deposit_tx_id, "Deposit expired"),
            Event::CommitFinalized { deposit_tx_id } => info!(%deposit_tx_id, "Commit finalized"),
            Event::TxValid { .. } | Event::TxInvalid { .. } => {}
        }

        if meta.seq >= self.progress.seq && !meta.timestamp.is_empty() {
            self.progress = Progress {
                seq: meta.seq,
                timestamp: meta.timestamp.clone(),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> (Event, EventMeta) {
        (
            serde_json::from_str(text).unwrap(),
            serde_json::from_str(text).unwrap(),
        )
    }

    fn apply_fixture(state: &mut HeadState, text: &str) {
        let (event, meta) = parse(text);
        state.apply(&event, &meta);
    }

    #[test]
    fn directly_open_head_tracks_incremental_commit() {
        let mut state = HeadState::default();

        apply_fixture(&mut state, include_str!("test_data/head_is_open_2x.json"));
        assert_eq!(state.head_status, HeadStatus::Open);
        assert!(state.utxos.is_empty());

        apply_fixture(&mut state, include_str!("test_data/commit_recorded.json"));
        assert!(state.utxos.is_empty());

        apply_fixture(&mut state, include_str!("test_data/commit_approved.json"));
        assert!(state.utxos.contains_key("deposit#0"));

        apply_fixture(
            &mut state,
            include_str!("test_data/snapshot_confirmed_with_commit.json"),
        );
        assert!(state.utxos.contains_key("deposit#0"));

        apply_fixture(
            &mut state,
            include_str!("test_data/snapshot_confirmed_null_commit.json"),
        );
        assert!(state.utxos.contains_key("deposit#0"));

        let before = state.utxos.len();
        apply_fixture(
            &mut state,
            include_str!("test_data/commit_finalized_021.json"),
        );
        assert_eq!(state.utxos.len(), before);
    }

    #[test]
    fn recovery_removes_recovered_keys_only() {
        let mut state = HeadState::default();

        apply_fixture(&mut state, include_str!("test_data/commit_approved.json"));
        apply_fixture(&mut state, include_str!("test_data/greetings_open.json"));
        apply_fixture(&mut state, include_str!("test_data/commit_approved.json"));
        assert_eq!(state.utxos.len(), 2);

        apply_fixture(&mut state, include_str!("test_data/commit_recovered.json"));
        assert!(!state.utxos.contains_key("deposit#0"));
        assert!(state.utxos.contains_key("legacy#0"));

        let before = state.utxos.clone();
        apply_fixture(&mut state, include_str!("test_data/commit_recovered.json"));
        assert_eq!(
            state.utxos.keys().collect::<Vec<_>>(),
            before.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn commit_approved_is_idempotent() {
        let mut state = HeadState::default();

        apply_fixture(&mut state, include_str!("test_data/commit_approved.json"));
        let before = state.utxos.len();
        apply_fixture(&mut state, include_str!("test_data/commit_approved.json"));

        assert_eq!(state.utxos.len(), before);
    }

    #[test]
    fn snapshot_replace_drops_consumed_utxos() {
        let mut state = HeadState::default();

        apply_fixture(&mut state, include_str!("test_data/greetings_open.json"));
        apply_fixture(&mut state, include_str!("test_data/commit_approved.json"));
        assert_eq!(state.utxos.len(), 2);

        apply_fixture(
            &mut state,
            include_str!("test_data/snapshot_confirmed_legacy.json"),
        );
        assert_eq!(state.utxos.len(), 1);
        assert!(state.utxos.contains_key("legacy#0"));
    }

    #[test]
    fn progress_advances_on_every_event_without_regressing() {
        let mut state = HeadState::default();

        apply_fixture(&mut state, include_str!("test_data/head_is_open_2x.json"));
        assert_eq!(state.progress.seq, 3);

        apply_fixture(&mut state, include_str!("test_data/commit_approved.json"));
        assert_eq!(state.progress.seq, 10);

        let (event, meta) = parse(include_str!("test_data/tx_valid.json"));
        state.apply(
            &event,
            &EventMeta {
                seq: 9,
                timestamp: "2026-01-01T00:00:09Z".to_string(),
            },
        );
        assert_eq!(state.progress.seq, 10);

        state.apply(&event, &meta);
        assert_eq!(state.progress.seq, 15);
    }

    #[test]
    fn legacy_open_and_snapshot_behave_as_before() {
        let mut state = HeadState::default();

        apply_fixture(
            &mut state,
            include_str!("test_data/head_is_open_legacy.json"),
        );
        assert_eq!(state.head_status, HeadStatus::Open);
        assert!(state.utxos.contains_key("legacy#0"));

        apply_fixture(
            &mut state,
            include_str!("test_data/snapshot_confirmed_legacy.json"),
        );
        assert_eq!(state.utxos.len(), 1);
        assert!(state.utxos.contains_key("legacy#0"));
    }
}
