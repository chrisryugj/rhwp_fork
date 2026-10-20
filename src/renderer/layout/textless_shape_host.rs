//! 글 없는 개체 host의 줄과 별도로 출력한 개체의 실제 소유를 구분한다.

use crate::model::{control::Control, paragraph::Paragraph, shape::TextWrap};
use crate::renderer::pagination::{PageContent, PageItem};

pub(super) struct TextlessShapeHost {
    pub(super) ladder_candidate: bool,
    /// TAC 플래그가 있어도 별도 Shape 항목이 이미 개체 출력을 소유한다.
    /// 저장 사다리가 없으면 host의 현재 구성 줄만 한 번 전진시킨다.
    pub(super) separate_inline_object: bool,
}

pub(super) fn control_has_visible_inline_content(control: &Control) -> bool {
    match control {
        Control::Picture(picture) => picture.common.treat_as_char,
        Control::Shape(shape) => shape.common().treat_as_char,
        Control::Table(table) => table.common.treat_as_char,
        Control::Equation(equation) => equation.common.treat_as_char,
        Control::Form(form) => form.common.treat_as_char,
        _ => false,
    }
}

pub(super) fn classify(
    page: &PageContent,
    para: &Paragraph,
    para_index: usize,
) -> Option<TextlessShapeHost> {
    if super::para_has_visible_text(para) {
        return None;
    }
    let separate = |control_index| {
        page.column_contents.iter().any(|column| {
            column.items.iter().any(|item| {
                matches!(item, PageItem::Shape { para_index: owner, control_index: control }
                    if *owner == para_index && *control == control_index)
            })
        })
    };
    if para
        .controls
        .iter()
        .enumerate()
        .any(|(index, control)| control_has_visible_inline_content(control) && !separate(index))
    {
        return None;
    }
    let mut found = false;
    let mut ladder_candidate = false;
    let mut separate_inline_object = false;
    for (index, control) in para.controls.iter().enumerate() {
        if !separate(index) {
            continue;
        }
        let common = match control {
            Control::Picture(picture) => &picture.common,
            Control::Shape(shape) => shape.common(),
            // 장식 표만 Shape 경로를 소유한다. 줄 안 표는 별도 Table 소유자를 유지한다.
            Control::Table(table) if !table.common.treat_as_char => &table.common,
            _ => continue,
        };
        found = true;
        separate_inline_object |= common.treat_as_char;
        ladder_candidate |= common.treat_as_char
            || matches!(
                common.text_wrap,
                TextWrap::InFrontOfText
                    | TextWrap::BehindText
                    | TextWrap::Square
                    | TextWrap::Tight
                    | TextWrap::Through
                    | TextWrap::TopAndBottom
            );
    }
    found.then_some(TextlessShapeHost {
        ladder_candidate,
        separate_inline_object,
    })
}
