//! 보조 spacer와 저장 줄 상자가 소유한 실제 빈 줄 전진을 구분한다.

use super::{CellUnit, LayoutEngine};
use crate::model::{paragraph::LineSeg, table::Cell};

/// 연속된 저장 사다리의 앞뒤 원점으로 빈 줄 자체의 점유를 확인한다.
/// 형제 셀의 유무는 이 줄 상자의 흐름 소유를 바꾸지 않는다.
fn closed_stored_empty_line_owns_advance(cell: &Cell, unit: &CellUnit) -> bool {
    if cell.dirty_flag || unit.vis_start != 0 || unit.vis_end != 1 {
        return false;
    }
    let Some(previous) = unit
        .para_idx
        .checked_sub(1)
        .and_then(|index| cell.paragraphs.get(index))
    else {
        return false;
    };
    let Some(paragraph) = cell.paragraphs.get(unit.para_idx) else {
        return false;
    };
    let Some(next) = cell.paragraphs.get(unit.para_idx + 1) else {
        return false;
    };
    if [previous, paragraph, next].iter().any(|para| {
        !para.controls.is_empty()
            || para.stored_text_partition_is_dirty()
            || para.cell_format_vpos_dirty
    }) || !paragraph.text.trim().is_empty()
        || paragraph.line_segs.len() != 1
    {
        return false;
    }
    let (Some(before), Some(line), Some(after)) = (
        previous.line_segs.last(),
        paragraph.line_segs.first(),
        next.line_segs.first(),
    ) else {
        return false;
    };
    let authentic = |segment: &LineSeg| {
        segment.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY == 0
            && segment.line_height > 0
            && segment.line_spacing >= 0
            && segment.vertical_pos >= 0
    };
    let closes = |left: &LineSeg, right: &LineSeg| {
        (i64::from(left.vertical_pos) + i64::from(left.line_height) + i64::from(left.line_spacing)
            - i64::from(right.vertical_pos))
        .abs()
            <= 2
    };
    authentic(before)
        && authentic(line)
        && authentic(after)
        && closes(before, line)
        && closes(line, after)
}

impl LayoutEngine {
    /// 실제 저장 줄 상자는 글자가 없어도 높이를 소유한다. 보조 spacer의
    /// 무높이 소비 규칙을 원본 LINE_SEG 빈 줄에 적용하면 컷만 앞서 진행하고
    /// 배치는 그 줄을 다시 그려 표가 쪽을 넘는다.
    pub(super) fn empty_unit_owns_flow_line_box(
        cell: &crate::model::table::Cell,
        table: &crate::model::table::Table,
        unit: &CellUnit,
    ) -> bool {
        if !unit.empty_spacer || unit.height <= 0.0 || unit.vis_start >= unit.vis_end {
            return false;
        }
        let Some(para) = cell.paragraphs.get(unit.para_idx) else {
            return false;
        };
        // A genuine reflowed empty paragraph owns its composed line. Lack of
        // saved LineSegs is not evidence that its positive advance is a spacer.
        if crate::renderer::para_has_no_stored_line_segs(para) {
            return para.controls.is_empty();
        }
        let Some(next) = cell.paragraphs.get(unit.para_idx + 1) else {
            // 종료 host의 빈 꼬리는 자체 줄 높이만으로 흐름 소유를 증명하지 못한다.
            // 다만 저장 줄이 새 쪽 원점(vpos 0)에서 다시 시작했다면 한/글이 그 줄을 새 쪽
            // 조각에 놓았다는 기록이다(`21761835` 3쪽 행 20: 빈 꼬리 한 줄 + 위아래 패딩 =
            // 한/글 2020 PDF 17px).
            return !cell.dirty_flag
                && unit.page_frame_reset_before
                && unit.stored_frame_break_before
                && para.controls.is_empty()
                && !para.stored_text_partition_is_dirty()
                && !para.cell_format_vpos_dirty
                && matches!(
                    para.line_segs.as_slice(),
                    [line] if line.vertical_pos == 0
                        && line.line_height > 0
                        && line.tag & crate::model::paragraph::LineSeg::TAG_IMPLEMENTATION_PROPERTY == 0
                );
        };
        if !para.controls.is_empty() || !next.controls.is_empty() {
            return false;
        }
        // 원본 쪽 재시작의 빈 줄은 뒤 저장 줄과 같은 전진 상자를 소유한다.
        // 형제 셀이 없어도 이 소유를 잃고 무높이 spacer로 소비하지 않는다.
        if Self::stored_empty_restart_owns_advance(cell, unit) {
            return true;
        }
        // 앞뒤 저장 원점이 빈 줄의 전진을 닫으면 형제 셀 없이도 실제 줄 상자다.
        if closed_stored_empty_line_owns_advance(cell, unit) {
            return true;
        }
        if crate::renderer::para_has_no_stored_line_segs(next) {
            // A saved blank line followed by a reflowed paragraph has no next
            // stored origin. Preserve the existing exact incoming-slot proof.
            return cell.paragraphs[..unit.para_idx]
                .last()
                .and_then(|previous| previous.line_segs.last())
                .zip(para.line_segs.first())
                .is_some_and(|(previous, line)| {
                    let authentic = |segment: &crate::model::paragraph::LineSeg| {
                        segment.tag & crate::model::paragraph::LineSeg::TAG_IMPLEMENTATION_PROPERTY
                            == 0
                    };
                    authentic(previous)
                        && authentic(line)
                        && previous.line_height > 0
                        && line.line_height > 0
                        && (i64::from(previous.vertical_pos)
                            + i64::from(previous.line_height)
                            + i64::from(previous.line_spacing.max(0))
                            - i64::from(line.vertical_pos))
                        .abs()
                            <= 2
                });
        }
        let (Some(seg), Some(next_seg)) = (para.line_segs.last(), next.line_segs.first()) else {
            return false;
        };
        let authentic = |seg: &crate::model::paragraph::LineSeg| {
            seg.tag & crate::model::paragraph::LineSeg::TAG_IMPLEMENTATION_PROPERTY == 0
        };
        // 저장 줄 상자가 실제 다음 원점까지 전진해야 한다. 저장 높이만 존재하는
        // overlay/종료 spacer를 모두 예약하면 기존 거대 셀의 컷이 바뀐다.
        authentic(seg)
            && authentic(next_seg)
            && seg.line_height > 0
            && i64::from(next_seg.vertical_pos)
                >= i64::from(seg.vertical_pos) + i64::from(seg.line_height)
            && table
                .cells
                .iter()
                .filter(|other| {
                    other.row == cell.row && other.col != cell.col && other.row_span == 1
                })
                .any(|other| {
                    let mut previous = None;
                    other
                        .paragraphs
                        .iter()
                        .filter(|p| p.controls.is_empty())
                        .flat_map(|p| &p.line_segs)
                        .any(|line| {
                            let matched =
                                previous.is_some_and(|prior: &crate::model::paragraph::LineSeg| {
                                    authentic(prior)
                                        && authentic(line)
                                        && prior.vertical_pos == seg.vertical_pos
                                        && prior.line_height == seg.line_height
                                        && line.vertical_pos == next_seg.vertical_pos
                                });
                            previous = Some(line);
                            matched
                        })
                })
    }
}
