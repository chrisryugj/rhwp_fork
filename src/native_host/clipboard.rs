//! Bounded body-text clipboard adapters; the OS clipboard belongs to the host.
//!
//! HTML uses the core's existing formatting importer/exporter, not a browser or
//! a general HTML sanitizer. Objects/tables and scalar/logical inline affinity
//! are outside this surface. Empty HTML and empty text preserve selection/history.

use serde::{Deserialize, Serialize};

use crate::document_core::native_interaction::{NativeTextPosition, NativeTextSelection};
use crate::model::control::Control;
use crate::DocumentCore;

use super::editor::{NativeEditOutcome, NativeEditor};
use super::NativeHostError;

pub const MAX_CLIPBOARD_BYTES: usize = 1024 * 1024;
/// Bound the number of core split/reflow operations in one plain-text paste.
pub const MAX_CLIPBOARD_PARAGRAPHS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeClipboardPayload {
    pub plain_text: String,
    /// None for an empty selection or when the existing HTML exporter would
    /// omit an empty selected paragraph fragment. Plain text remains lossless.
    pub html: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HtmlPasteResult {
    ok: bool,
    para_idx: Option<u32>,
    char_offset: Option<u32>,
    error: Option<String>,
}

impl NativeEditor {
    /// Read-only, including revision/history/selection. Active preedit is not a
    /// committed clipboard selection and must be resolved by the host first.
    pub fn copy_selection(
        &self,
        expected_revision: u64,
    ) -> Result<NativeClipboardPayload, NativeHostError> {
        self.check_edit_ready(expected_revision)?;
        let selection = self.selection();
        let (bytes, empty_fragment) =
            clipboard_selection_size(self.session().document(), selection)?;
        if bytes == 0 {
            return Ok(NativeClipboardPayload {
                plain_text: String::new(),
                html: None,
            });
        }
        let plain_text = self.selected_text(selection);
        check_text_controls(&plain_text)?;
        let (start, end) = Self::ordered(selection);
        let html = if empty_fragment {
            None
        } else {
            Some(self.session().document().export_selection_html_native(
                start.section as usize,
                start.paragraph as usize,
                start.scalar_offset as usize,
                end.paragraph as usize,
                end.scalar_offset as usize,
            )?)
        };
        if plain_text
            .len()
            .checked_add(html.as_ref().map_or(0, String::len))
            .is_none_or(|size| size > MAX_CLIPBOARD_BYTES)
        {
            return Err(clipboard_limit());
        }
        Ok(NativeClipboardPayload { plain_text, html })
    }

    /// Payload construction completes before deletion, so export/limit failures
    /// cannot cut content. The returned payload is for host clipboard ownership.
    pub fn cut_selection(
        &mut self,
        expected_revision: u64,
    ) -> Result<(NativeClipboardPayload, NativeEditOutcome), NativeHostError> {
        let payload = self.copy_selection(expected_revision)?;
        let outcome = self.replace_selection(expected_revision, "")?;
        Ok((payload, outcome))
    }

    pub fn paste_text(
        &mut self,
        expected_revision: u64,
        text: &str,
    ) -> Result<NativeEditOutcome, NativeHostError> {
        self.check_edit_ready(expected_revision)?;
        check_input_size(text)?;
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        check_text_controls(&text)?;
        if text.is_empty() {
            return self.set_selection(expected_revision, self.selection().clone());
        }
        let lines: Vec<&str> = text
            .split('\n')
            .take(MAX_CLIPBOARD_PARAGRAPHS + 1)
            .collect();
        if lines.len() > MAX_CLIPBOARD_PARAGRAPHS {
            return Err(NativeHostError::LimitExceeded(
                "clipboard paragraph limit exceeded".into(),
            ));
        }
        let selection = self.selection().clone();
        if lines.len() == 1 {
            return self.replace_range(expected_revision, &selection, &text);
        }
        let (start, end) = Self::ordered(&selection);
        // The comparison allocates selected text only after its byte bound is
        // known; replacing a larger selection with a small payload is allowed.
        if clipboard_selection_size(self.session().document(), &selection)
            .ok()
            .is_some_and(|(bytes, _)| bytes == text.len())
            && self.selected_text(&selection) == text
        {
            return self.set_selection(
                expected_revision,
                NativeTextSelection {
                    anchor: end.clone(),
                    focus: end.clone(),
                },
            );
        }
        self.edit_transaction(expected_revision, |core| {
            Self::apply_replacement(core, &selection, lines[0])?;
            let mut focus = start.clone();
            focus.scalar_offset = focus
                .scalar_offset
                .checked_add(lines[0].chars().count() as u32)
                .ok_or_else(clipboard_limit)?;
            for line in &lines[1..] {
                core.split_paragraph_native(
                    focus.section as usize,
                    focus.paragraph as usize,
                    focus.scalar_offset as usize,
                    None,
                )?;
                focus.paragraph = focus.paragraph.checked_add(1).ok_or_else(clipboard_limit)?;
                focus.scalar_offset = 0;
                if !line.is_empty() {
                    core.insert_text_native(
                        focus.section as usize,
                        focus.paragraph as usize,
                        0,
                        line,
                    )?;
                }
                focus.scalar_offset = line.chars().count() as u32;
            }
            Ok(NativeTextSelection {
                anchor: focus.clone(),
                focus,
            })
        })
    }

    pub fn paste_html(
        &mut self,
        expected_revision: u64,
        html: &str,
    ) -> Result<NativeEditOutcome, NativeHostError> {
        self.check_edit_ready(expected_revision)?;
        check_input_size(html)?;
        let selection = self.selection().clone();
        if html.trim().is_empty() {
            return self.set_selection(expected_revision, selection);
        }
        let (start, end) = Self::ordered(&selection);
        check_imported_paragraphs(self.session().document(), start, end)?;
        let mut empty_html = false;
        let result = self.edit_transaction(expected_revision, |core| {
            if start != end {
                Self::apply_replacement(core, &selection, "")?;
            }
            let before_count = core.document().sections[start.section as usize]
                .paragraphs
                .len();
            let json = core.paste_html_native(
                start.section as usize,
                start.paragraph as usize,
                start.scalar_offset as usize,
                html,
            )?;
            let imported: HtmlPasteResult = serde_json::from_str(&json).map_err(|_| {
                NativeHostError::InvalidArgument(
                    "HTML importer returned an invalid caret response".into(),
                )
            })?;
            if !imported.ok {
                if imported.error.as_deref() == Some("empty html")
                    && imported.para_idx.is_none()
                    && imported.char_offset.is_none()
                {
                    empty_html = true;
                }
                return Err(NativeHostError::InvalidArgument(
                    "HTML contained no supported content".into(),
                ));
            }
            if imported.error.is_some() {
                return Err(NativeHostError::InvalidArgument(
                    "HTML importer returned conflicting status".into(),
                ));
            }
            let mut focus = start.clone();
            focus.paragraph = imported.para_idx.ok_or_else(invalid_import_caret)?;
            focus.scalar_offset = imported.char_offset.ok_or_else(invalid_import_caret)?;
            let after_count = core.document().sections[start.section as usize]
                .paragraphs
                .len();
            let added = after_count
                .checked_sub(before_count)
                .ok_or_else(invalid_import_caret)?;
            if added >= MAX_CLIPBOARD_PARAGRAPHS {
                return Err(NativeHostError::LimitExceeded(
                    "clipboard paragraph limit exceeded".into(),
                ));
            }
            let last = (start.paragraph as usize)
                .checked_add(added)
                .ok_or_else(invalid_import_caret)?;
            if focus.paragraph as usize != last {
                return Err(invalid_import_caret());
            }
            // Validate every inserted paragraph, not only the final caret: the
            // importer may put a table between two otherwise valid body lines.
            check_imported_paragraphs(core, start, &focus)?;
            Self::validate_selection(
                core,
                &NativeTextSelection {
                    anchor: start.clone(),
                    focus: focus.clone(),
                },
            )?;
            if added == 0 && focus == *start {
                // <p></p> can return success after interning styles. Roll back
                // those changes and any prior selection deletion as a no-op.
                empty_html = true;
                return Err(NativeHostError::InvalidArgument(
                    "HTML inserted no content".into(),
                ));
            }
            Ok(NativeTextSelection {
                anchor: focus.clone(),
                focus,
            })
        });
        match result {
            // A failed snapshot restore is Core, not InvalidArgument, and is
            // propagated rather than misreported as a successful no-op.
            Err(NativeHostError::InvalidArgument(_)) if empty_html => {
                self.set_selection(expected_revision, selection)
            }
            result => result,
        }
    }
}

fn check_input_size(text: &str) -> Result<(), NativeHostError> {
    if text.len() > MAX_CLIPBOARD_BYTES {
        Err(clipboard_limit())
    } else {
        Ok(())
    }
}

fn clipboard_limit() -> NativeHostError {
    NativeHostError::LimitExceeded("clipboard payload exceeds 1 MiB".into())
}

fn invalid_import_caret() -> NativeHostError {
    NativeHostError::InvalidArgument("HTML importer returned an invalid scalar caret".into())
}

fn check_text_controls(text: &str) -> Result<(), NativeHostError> {
    if text.chars().any(|ch| ch.is_control() && ch != '\n') {
        return Err(NativeHostError::Unsupported(
            "clipboard tabs and control characters are unsupported".into(),
        ));
    }
    Ok(())
}

fn clipboard_selection_size(
    core: &DocumentCore,
    selection: &NativeTextSelection,
) -> Result<(usize, bool), NativeHostError> {
    let (start, end) = NativeEditor::ordered(selection);
    let paragraphs = &core.document().sections[start.section as usize].paragraphs;
    let mut bytes = (end.paragraph - start.paragraph) as usize;
    let mut empty_fragment = false;
    if bytes >= MAX_CLIPBOARD_PARAGRAPHS {
        return Err(NativeHostError::LimitExceeded(
            "clipboard paragraph limit exceeded".into(),
        ));
    }
    for index in start.paragraph..=end.paragraph {
        let text = &paragraphs[index as usize].text;
        let from = if index == start.paragraph {
            start.scalar_offset as usize
        } else {
            0
        };
        let count = if index == end.paragraph {
            (end.scalar_offset as usize) - from
        } else {
            usize::MAX
        };
        let mut fragment_bytes = 0;
        for ch in text.chars().skip(from).take(count) {
            if ch.is_control() {
                return Err(NativeHostError::Unsupported(
                    "clipboard paragraph contains unsupported control characters".into(),
                ));
            }
            fragment_bytes += ch.len_utf8();
            bytes = bytes
                .checked_add(ch.len_utf8())
                .ok_or_else(clipboard_limit)?;
            if bytes > MAX_CLIPBOARD_BYTES {
                return Err(clipboard_limit());
            }
        }
        empty_fragment |= fragment_bytes == 0;
    }
    Ok((bytes, empty_fragment))
}

fn check_imported_paragraphs(
    core: &DocumentCore,
    start: &NativeTextPosition,
    end: &NativeTextPosition,
) -> Result<(), NativeHostError> {
    for paragraph in &core.document().sections[start.section as usize].paragraphs
        [start.paragraph as usize..=end.paragraph as usize]
    {
        if paragraph.controls.iter().any(|control| {
            control.is_logical_inline()
                || matches!(
                    control,
                    Control::Table(_)
                        | Control::Shape(_)
                        | Control::Picture(_)
                        | Control::Equation(_)
                        | Control::Form(_)
                        | Control::CharOverlap(_)
                )
        }) {
            return Err(NativeHostError::Unsupported(
                "clipboard object and table import is unsupported".into(),
            ));
        }
        if paragraph.text.chars().any(char::is_control) {
            return Err(NativeHostError::Unsupported(
                "clipboard paragraph contains unsupported control characters".into(),
            ));
        }
    }
    Ok(())
}
