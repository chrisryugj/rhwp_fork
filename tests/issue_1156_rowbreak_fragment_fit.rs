//! RowBreak 표의 내용 순서와 조각의 실제 본문 포함을 검사한다.
//!
//! 독립 Print 37쪽은 성능 요구사항으로 끝나고 38쪽은 데이터 요구사항으로
//! 이어진다. 기존 rows=0..16 핀은 데이터 행까지 앞 쪽에 수용하도록 강제했다.
//! 행 번호 핀 대신 두 조각의 원본 행 소유·내용 순서·본문 점유를 검사한다.

use std::fs;
use std::path::Path;

fn load_doc(rel_path: &str) -> rhwp::wasm_api::HwpDocument {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(repo_root).join(rel_path);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {rel_path}: {e}"));
    rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
        .unwrap_or_else(|e| panic!("parse {rel_path}: {e:?}"))
}

fn page_dump(rel_path: &str, page_idx: u32) -> String {
    let doc = load_doc(rel_path);
    doc.dump_page_items(Some(page_idx))
}

fn subtree_text(node: &rhwp::renderer::render_tree::RenderNode) -> String {
    use rhwp::renderer::render_tree::RenderNodeType;
    let own = match &node.node_type {
        RenderNodeType::TextRun(run) => run.display_or_text().to_string(),
        _ => String::new(),
    };
    node.children.iter().fold(own, |mut text, child| {
        text.push_str(&subtree_text(child));
        text
    })
}

fn table_with_label<'a>(
    node: &'a rhwp::renderer::render_tree::RenderNode,
    label: &str,
) -> Option<&'a rhwp::renderer::render_tree::RenderNode> {
    use rhwp::renderer::render_tree::RenderNodeType;
    if matches!(node.node_type, RenderNodeType::Table(_)) && subtree_text(node).contains(label) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| table_with_label(child, label))
}

#[test]
fn kps_ai_page37_defers_overflowing_split_row_slice() {
    use rhwp::document_core::DocumentCore;
    use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
    use std::collections::BTreeSet;

    fn left_rows(table: &RenderNode) -> BTreeSet<u16> {
        table
            .children
            .iter()
            .filter_map(|node| match &node.node_type {
                RenderNodeType::TableCell(cell) if cell.col == 0 => Some(cell.row),
                _ => None,
            })
            .collect()
    }
    fn column_contains_table(root: &RenderNode, table: &RenderNode) -> bool {
        matches!(root.node_type, RenderNodeType::Column(_))
            && root.children.iter().any(|child| std::ptr::eq(child, table))
            && table.bbox.y >= root.bbox.y - 0.5
            && table.bbox.y + table.bbox.height <= root.bbox.y + root.bbox.height + 0.5
            || root
                .children
                .iter()
                .any(|child| column_contains_table(child, table))
    }

    let bytes = fs::read("samples/kps-ai.hwp").expect("read sample");
    let core = DocumentCore::from_bytes(&bytes).expect("parse sample");
    let pages: Vec<_> = (0..38)
        .map(|page| core.build_page_render_tree(page).expect("render page"))
        .collect();
    let before = &pages[36].root;
    let after = &pages[37].root;
    let first = table_with_label(before, "성능 요구사항").expect("performance row on page 37");
    let next = table_with_label(after, "데이터 요구사항").expect("data row on page 38");
    assert!(
        !subtree_text(first).contains("데이터 요구사항"),
        "data row belongs to next page"
    );
    assert!(
        !subtree_text(next).contains("성능 요구사항"),
        "performance row must not repeat"
    );
    assert!(
        column_contains_table(before, first),
        "opening fragment escapes body"
    );
    assert!(
        column_contains_table(after, next),
        "continuation fragment escapes body"
    );
    let (RenderNodeType::Table(first_info), RenderNodeType::Table(next_info)) =
        (&first.node_type, &next.node_type)
    else {
        unreachable!("table_with_label returns a table");
    };
    assert_eq!(
        first_info.para_index, next_info.para_index,
        "same source table owner"
    );
    assert_eq!(
        first_info.control_index, next_info.control_index,
        "same source control owner"
    );
    let rows = left_rows(first);
    let following = left_rows(next);
    let headers: BTreeSet<_> = first
        .children
        .iter()
        .filter_map(|node| match &node.node_type {
            RenderNodeType::TableCell(cell)
                if cell.col == 0
                    && subtree_text(node).split_whitespace().collect::<String>() == "항목" =>
            {
                Some(cell.row)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        rows.intersection(&following)
            .copied()
            .collect::<BTreeSet<_>>(),
        headers,
        "only the repeated header may share row ownership"
    );
    assert_eq!(
        rows.union(&following).count(),
        usize::from(first_info.row_count),
        "two fragments must preserve every original row"
    );
}

#[test]
fn synam_001_page14_uses_row_budget_after_repeated_header() {
    let sample = "samples/synam-001.hwp";

    let page13 = page_dump(sample, 12);
    assert!(
        page13.contains("PartialTable   pi=140 ci=0  rows=2..7"),
        "page 13 should include the first visible slice of row 6 after repeated header rows:\n{page13}"
    );
    assert!(
        page13.contains("start_cut=[2, 19] end_cut=[2, 13]"),
        "page 13 should continue row 2 and cut into row 6 instead of deferring row 6:\n{page13}"
    );

    let page14 = page_dump(sample, 13);
    assert!(
        page14.contains("start_cut=[2, 13]"),
        "page 14 should continue from the row 6 cut already started on page 13:\n{page14}"
    );
}
