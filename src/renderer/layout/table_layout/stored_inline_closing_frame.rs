//! 저장된 마지막 그림 글줄 프레임의 물리 높이.

use super::{LayoutEngine, ResolvedStyleSet};
use crate::model::control::Control;
use crate::model::paragraph::LineSeg;
use crate::model::table::{Table, TablePageBreak};
use crate::renderer::hwpunit_to_px;

impl LayoutEngine {
    /// 일반 행의 마지막 원본 프레임이 그림 글줄로 다시 시작한 경우다.
    /// 앞 프레임의 줄 전진 합이나 행 전체의 최소를 현재 프레임에 재할당하지 않는다.
    /// 이 높이는 다음 컷의 측정과 실제 배치가 함께 소비하는 이어받기 상자가 된다.
    pub(crate) fn stored_inline_picture_closing_row_height(
        &self,
        table: &Table,
        row: usize,
        cut: &[usize],
        styles: &ResolvedStyleSet,
    ) -> Option<f64> {
        let profile = self.profile.get();
        if !(profile.hwp5_stored_pagination_layout() || profile.hwpx_stored_layout())
            || profile.session_edited()
            || self
                .render_normalization
                .borrow()
                .table_text_reflowed(table)
            || table.common.treat_as_char
            || table.page_break != TablePageBreak::RowBreak
            || !self.stored_row_boxes_exceed_first_frame(table)
            || table.cells.iter().any(|cell| {
                cell.row_span > 1
                    && usize::from(cell.row) <= row
                    && row < usize::from(cell.row) + usize::from(cell.row_span)
            })
        {
            return None;
        }
        let indices = Self::row_cut_cell_order(table, row);
        if indices.is_empty() || indices.len() != cut.len() {
            return None;
        }
        let mut height = 0.0_f64;
        let mut has_picture = false;
        for (&index, &start) in indices.iter().zip(cut) {
            let cell = table.cells.get(index)?;
            if cell.row as usize != row || cell.row_span != 1 || cell.dirty_flag {
                return None;
            }
            // 지원하는 그림은 자체 저장 줄에 든 TAC 개체뿐이다. 떠 있는 그림,
            // 여러 개체·글이 섞인 줄과 중첩 표는 별도의 소유 계약을 유지한다.
            for para in &cell.paragraphs {
                if para.stored_text_partition_is_dirty()
                    || para.cell_format_vpos_dirty
                    || para.line_segs.is_empty()
                    || para.line_segs.iter().any(|line| {
                        line.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY != 0
                            || line.vertical_pos < 0
                            || line.line_height <= 0
                            || line.line_spacing < 0
                    })
                {
                    return None;
                }
                if !para.controls.is_empty() {
                    let ([Control::Picture(picture)], [line]) =
                        (para.controls.as_slice(), para.line_segs.as_slice())
                    else {
                        return None;
                    };
                    if !picture.common.treat_as_char
                        || !para.text.trim().is_empty()
                        || picture.common.vertical_offset != 0
                        || picture.common.height > line.line_height as u32
                        || picture.common.margin.top != 0
                        || picture.common.margin.bottom != 0
                    {
                        return None;
                    }
                }
            }
            let units = self.cell_units(cell, table, styles);
            if start == units.len() {
                continue;
            }
            let first = units.get(start)?;
            let para = cell.paragraphs.get(first.para_idx)?;
            let previous = cell.paragraphs.get(first.para_idx.checked_sub(1)?)?;
            let opening = para.line_segs.first()?;
            let previous_first = previous.line_segs.first()?;
            let previous_last = previous.line_segs.last()?;
            if first.vis_start != 0
                || opening.vertical_pos != 0
                || previous_first.vertical_pos <= 0
                || previous_last.vertical_pos <= 0
                || opening.column_start != previous_last.column_start
                || self.cell_cut_has_projected_lines(cell, table, styles, start, units.len())
            {
                return None;
            }
            let range = Self::cell_cut_flow_range(cell, table, &units, start, units.len());
            let mut previous_top = 0;
            let mut bottom = 0;
            for unit in &units[range] {
                let para = cell.paragraphs.get(unit.para_idx)?;
                has_picture |= para
                    .controls
                    .iter()
                    .any(|control| matches!(control, Control::Picture(_)));
                for line in para.line_segs.get(unit.vis_start..unit.vis_end)? {
                    if line.vertical_pos < previous_top {
                        return None;
                    }
                    previous_top = line.vertical_pos;
                    bottom = bottom.max(line.vertical_pos.checked_add(line.line_height)?);
                }
            }
            let padding = cell.effective_padding(&table.padding);
            let physical = bottom
                .checked_add(i32::from(padding.top))?
                .checked_add(i32::from(padding.bottom))?;
            height = height.max(hwpunit_to_px(physical, self.dpi));
        }
        (has_picture && height > 0.0).then_some(height)
    }
}
