use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use crate::i18n::{tf, tr, Key};

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SegmentState {
    #[default]
    Partial,
    Final,
    Edited,
}

impl SegmentState {
    pub fn is_frozen(self) -> bool {
        matches!(self, SegmentState::Final | SegmentState::Edited)
    }

    pub fn is_persistable(self) -> bool {
        self.is_frozen()
    }
}

#[typeshare]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TranslationState {
    #[default]
    Idle,
    Pending,
    Done,
    Failed,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    #[typeshare(serialized_as = "number")]
    pub start_ms: u64,
    #[typeshare(serialized_as = "number")]
    pub end_ms: u64,
    pub text: String,
    pub translation: Option<String>,
    pub translation_state: TranslationState,
    pub state: SegmentState,
    pub revision: u32,
}

impl TranscriptSegment {
    pub fn is_persistable(&self) -> bool {
        self.state.is_persistable()
    }

    pub fn has_translation(&self) -> bool {
        self.translation_state == TranslationState::Done && self.translation.is_some()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("段落 {id} 已冻结（当前状态 {current:?}），MUST NOT 被引擎输出覆盖")]
    FrozenSegment { id: String, current: SegmentState },

    #[error("段落 {id} 尚未定稿（当前状态 {current:?}），不可编辑")]
    NotFinalized { id: String, current: SegmentState },

    #[error("段落 {id} 不存在")]
    UnknownSegment { id: String },
}

#[derive(Debug, Default)]
pub struct TranscriptStore {
    segments: Vec<TranscriptSegment>,
    index: HashMap<String, usize>,
}

impl TranscriptStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(
        &mut self,
        delta: Vec<TranscriptSegment>,
    ) -> Result<Vec<TranscriptSegment>, StoreError> {
        let mut changed = Vec::new();

        for incoming in delta {
            match self.index.get(&incoming.id).copied() {
                None => {
                    self.index.insert(incoming.id.clone(), self.segments.len());
                    changed.push(incoming.clone());
                    self.segments.push(incoming);
                }
                Some(pos) => {
                    if self.segments[pos].state.is_frozen() {
                        return Err(StoreError::FrozenSegment {
                            id: incoming.id,
                            current: self.segments[pos].state,
                        });
                    }

                    if incoming.revision <= self.segments[pos].revision {
                        continue;
                    }

                    self.segments[pos] = incoming;
                    changed.push(self.segments[pos].clone());
                }
            }
        }

        Ok(changed)
    }

    pub fn mark_pending_translation(&mut self, ids: &[String]) -> Vec<TranscriptSegment> {
        let mut changed = Vec::new();
        for id in ids {
            if let Some(&pos) = self.index.get(id) {
                let seg = &mut self.segments[pos];
                if seg.state.is_frozen() && seg.translation_state == TranslationState::Idle {
                    seg.translation_state = TranslationState::Pending;
                    seg.revision += 1;
                    changed.push(seg.clone());
                }
            }
        }
        changed
    }

    pub fn set_translation(
        &mut self,
        id: &str,
        result: Result<String, String>,
    ) -> Result<TranscriptSegment, StoreError> {
        let pos = *self
            .index
            .get(id)
            .ok_or_else(|| StoreError::UnknownSegment { id: id.to_string() })?;

        let seg = &mut self.segments[pos];
        match result {
            Ok(text) => {
                seg.translation = Some(text);
                seg.translation_state = TranslationState::Done;
            }
            Err(reason) => {
                tracing::warn!(id, "翻译失败：{reason}");
                seg.translation = None;
                seg.translation_state = TranslationState::Failed;
            }
        }
        seg.revision += 1;
        Ok(seg.clone())
    }

    pub fn edit(&mut self, id: &str, text: String) -> Result<u32, StoreError> {
        let pos = *self
            .index
            .get(id)
            .ok_or_else(|| StoreError::UnknownSegment { id: id.to_string() })?;

        let existing = &self.segments[pos];
        if !existing.state.is_frozen() {
            return Err(StoreError::NotFinalized {
                id: id.to_string(),
                current: existing.state,
            });
        }

        let seg = &mut self.segments[pos];
        seg.text = text;
        seg.state = SegmentState::Edited;
        seg.revision += 1;
        Ok(seg.revision)
    }

    pub fn snapshot(&self) -> &[TranscriptSegment] {
        &self.segments
    }

    pub fn persistable(&self) -> impl Iterator<Item = &TranscriptSegment> {
        self.segments.iter().filter(|s| s.is_persistable())
    }

    pub fn get(&self, id: &str) -> Option<&TranscriptSegment> {
        self.index.get(id).map(|&pos| &self.segments[pos])
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
}

pub fn to_markdown(
    segments: &[TranscriptSegment],
    exported_at: &str,
    direction: Option<&str>,
) -> String {
    let mut out = format!("{}\n\n", tr(Key::ExportTitle));
    out.push_str(&format!("{}\n", tf(Key::ExportTime, &[exported_at])));
    if let Some(d) = direction {
        out.push_str(&format!("{}\n", tf(Key::ExportDirection, &[d])));
    }

    let settled: Vec<&TranscriptSegment> = segments.iter().filter(|s| s.is_persistable()).collect();
    out.push_str(&format!(
        "{}\n\n---\n\n",
        tf(Key::ExportSegments, &[&settled.len().to_string()])
    ));

    for s in settled {
        out.push_str(&format!(
            "**[{}]** {}\n\n",
            clock(s.start_ms),
            s.text.trim()
        ));
        if let Some(t) = s
            .translation
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            out.push_str(&format!("> {t}\n\n"));
        }
    }
    out
}

fn clock(ms: u64) -> String {
    let total = ms / 1000;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(id: &str, text: &str, state: SegmentState, revision: u32) -> TranscriptSegment {
        TranscriptSegment {
            id: id.into(),
            start_ms: 0,
            end_ms: 500,
            text: text.into(),
            translation: None,
            translation_state: TranslationState::Idle,
            state,
            revision,
        }
    }

    #[test]
    fn partial_never_gets_queued_for_translation() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "还在变", SegmentState::Partial, 1)])
            .unwrap();

        let changed = s.mark_pending_translation(&["a".to_string()]);
        assert!(changed.is_empty(), "partial 不该被翻译");
        assert_eq!(
            s.get("a").unwrap().translation_state,
            TranslationState::Idle
        );
    }

    #[test]
    fn final_gets_queued_then_translated() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "hello world", SegmentState::Final, 1)])
            .unwrap();

        let queued = s.mark_pending_translation(&["a".to_string()]);
        assert_eq!(queued.len(), 1);
        assert_eq!(
            s.get("a").unwrap().translation_state,
            TranslationState::Pending
        );

        let updated = s.set_translation("a", Ok("你好世界".into())).unwrap();
        assert_eq!(updated.translation.as_deref(), Some("你好世界"));
        assert_eq!(updated.translation_state, TranslationState::Done);
        assert!(updated.has_translation());
    }

    #[test]
    fn translation_arrival_bumps_revision() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "hello", SegmentState::Final, 1)])
            .unwrap();
        let before = s.get("a").unwrap().revision;
        let after = s.set_translation("a", Ok("你好".into())).unwrap().revision;
        assert!(after > before, "revision 必须递增：{before} → {after}");
    }

    #[test]
    fn translation_failure_is_recorded_not_swallowed() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "hello", SegmentState::Final, 1)])
            .unwrap();
        s.mark_pending_translation(&["a".to_string()]);

        s.set_translation("a", Err("模型未加载".into())).unwrap();
        let seg = s.get("a").unwrap();
        assert_eq!(seg.translation_state, TranslationState::Failed);
        assert!(!seg.has_translation());
    }

    #[test]
    fn already_translated_segment_is_not_requeued() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "hello", SegmentState::Final, 1)])
            .unwrap();
        s.mark_pending_translation(&["a".to_string()]);
        s.set_translation("a", Ok("你好".into())).unwrap();

        let again = s.mark_pending_translation(&["a".to_string()]);
        assert!(again.is_empty());
        assert!(s.get("a").unwrap().has_translation());
    }

    #[test]
    fn partial_can_be_overwritten() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "你好", SegmentState::Partial, 1)])
            .unwrap();
        s.apply(vec![seg("a", "你好世界", SegmentState::Partial, 2)])
            .unwrap();
        assert_eq!(s.get("a").unwrap().text, "你好世界");
        assert_eq!(s.segment_count(), 1, "覆盖不应产生新段");
    }

    #[test]
    fn same_id_survives_partial_to_final() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "今天下午", SegmentState::Partial, 1)])
            .unwrap();
        s.apply(vec![seg("a", "今天下午三点开会", SegmentState::Final, 2)])
            .unwrap();

        assert_eq!(s.segment_count(), 1);
        assert_eq!(s.get("a").unwrap().state, SegmentState::Final);
        assert_eq!(s.get("a").unwrap().text, "今天下午三点开会");
    }

    #[test]
    fn final_is_frozen_against_later_engine_output() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "定稿文本", SegmentState::Final, 2)])
            .unwrap();

        let err = s
            .apply(vec![seg("a", "引擎想改", SegmentState::Partial, 3)])
            .unwrap_err();
        assert!(matches!(err, StoreError::FrozenSegment { .. }));
        assert_eq!(s.get("a").unwrap().text, "定稿文本");
    }

    #[test]
    fn final_must_not_recur_with_same_id() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "第一次定稿", SegmentState::Final, 1)])
            .unwrap();

        let err = s
            .apply(vec![seg("a", "第二次定稿", SegmentState::Final, 2)])
            .unwrap_err();
        assert!(matches!(err, StoreError::FrozenSegment { .. }));
    }

    #[test]
    fn edited_is_permanently_locked() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "引擎文本", SegmentState::Final, 1)])
            .unwrap();
        s.edit("a", "用户改过的文本".into()).unwrap();

        assert_eq!(s.get("a").unwrap().state, SegmentState::Edited);
        let err = s
            .apply(vec![seg("a", "引擎又想改", SegmentState::Partial, 9)])
            .unwrap_err();
        assert!(matches!(err, StoreError::FrozenSegment { .. }));
        assert_eq!(s.get("a").unwrap().text, "用户改过的文本");
    }

    #[test]
    fn cannot_edit_a_partial() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "还在变", SegmentState::Partial, 1)])
            .unwrap();
        let err = s.edit("a", "改".into()).unwrap_err();
        assert!(matches!(err, StoreError::NotFinalized { .. }));
    }

    #[test]
    fn one_delta_may_carry_a_final_and_a_new_partial() {
        let mut s = TranscriptStore::new();
        s.apply(vec![
            seg("a", "第一句。", SegmentState::Final, 2),
            seg("b", "第二句", SegmentState::Partial, 1),
        ])
        .unwrap();

        assert_eq!(s.segment_count(), 2);
        assert_eq!(s.get("a").unwrap().state, SegmentState::Final);
        assert_eq!(s.get("b").unwrap().state, SegmentState::Partial);
    }

    #[test]
    fn stale_revision_is_ignored_not_an_error() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "新", SegmentState::Partial, 5)])
            .unwrap();
        s.apply(vec![seg("a", "旧", SegmentState::Partial, 3)])
            .unwrap();
        assert_eq!(s.get("a").unwrap().text, "新");
    }

    #[test]
    fn apply_returns_the_merged_segments_for_broadcast() {
        let mut s = TranscriptStore::new();
        let out = s
            .apply(vec![seg("a", "你好", SegmentState::Partial, 1)])
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "你好");
        assert_eq!(out[0].state, SegmentState::Partial);

        let out = s
            .apply(vec![seg("a", "旧的", SegmentState::Partial, 0)])
            .unwrap();
        assert!(out.is_empty(), "被拒的增量不该被广播");
    }

    #[test]
    fn partial_never_persists() {
        let mut s = TranscriptStore::new();
        s.apply(vec![
            seg("a", "草稿", SegmentState::Partial, 1),
            seg("b", "定稿", SegmentState::Final, 1),
        ])
        .unwrap();
        s.edit("b", "定稿（改）".into()).unwrap();

        let persisted: Vec<_> = s.persistable().map(|x| x.id.as_str()).collect();
        assert_eq!(persisted, vec!["b"], "partial 必须被排除在落盘之外");
    }

    #[test]
    fn edit_bumps_revision() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "原", SegmentState::Final, 1)])
            .unwrap();
        let rev = s.edit("a", "改".into()).unwrap();
        assert_eq!(rev, 2, "编辑 MUST 递增 revision，否则前端不会重绘");
    }

    #[test]
    fn markdown_excludes_partial_and_quotes_translation() {
        let mut s = TranscriptStore::new();
        s.apply(vec![
            seg("a", "已经定稿的一句。", SegmentState::Final, 1),
            seg("b", "还在改写中", SegmentState::Partial, 1),
        ])
        .unwrap();
        s.mark_pending_translation(&["a".to_string()]);
        s.set_translation("a", Ok("A finished sentence.".into()))
            .unwrap();

        let md = to_markdown(s.snapshot(), "2026-10-16 10:23", Some("英语 → 中文"));

        assert!(md.contains("**[00:00]** 已经定稿的一句。"));
        assert!(md.contains("> A finished sentence."), "译文应引作注解");
        assert!(!md.contains("还在改写中"), "partial 不该进导出");
        assert!(md.contains("- 翻译方向：英语 → 中文"));
        assert!(md.contains("- 段落数：1"));
    }

    #[test]
    fn markdown_omits_direction_when_translation_is_off() {
        let mut s = TranscriptStore::new();
        s.apply(vec![seg("a", "只有原文", SegmentState::Final, 1)])
            .unwrap();

        let md = to_markdown(s.snapshot(), "2026-10-16 10:23", None);
        assert!(!md.contains("翻译方向"), "关了翻译就不该编出一行方向");
        assert!(md.contains("只有原文"));
    }

    #[test]
    fn clock_adds_hours_only_past_an_hour() {
        assert_eq!(clock(0), "00:00");
        assert_eq!(clock(9_000), "00:09");
        assert_eq!(clock(3_599_000), "59:59");
        assert_eq!(clock(3_600_000), "1:00:00");
    }
}
