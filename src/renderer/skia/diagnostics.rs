//! Per-render observations, kept separate from selection and cache authority.

use crate::paint::{PageLayerTree, RenderProfile, TextVariantKind};
use crate::renderer::layer_renderer::{
    LayerRenderDiagnostics, TextVariantRejectReport, TextVariantSelectionReport,
    VariantRejectReason, VariantSelectedReason, VariantSelectionBackend,
};

use super::cache::NativeSkiaCacheStats;
use super::image_cache::ImageReplayCounters;

pub(super) struct ReplayDiagnostics {
    pub output: LayerRenderDiagnostics,
    pub selection_updates: Vec<TextVariantSelectionReport>,
}

impl ReplayDiagnostics {
    pub fn new(tree: &PageLayerTree) -> Self {
        Self {
            output: LayerRenderDiagnostics {
                backend: Some(VariantSelectionBackend::NativeSkia),
                render_profile: Some(
                    match tree.profile {
                        RenderProfile::FastPreview => "fastPreview",
                        RenderProfile::Screen => "screen",
                        RenderProfile::Print => "print",
                        RenderProfile::HighQuality => "highQuality",
                    }
                    .into(),
                ),
                ..Default::default()
            },
            selection_updates: Vec::new(),
        }
    }

    pub fn apply_selection(&mut self, report: TextVariantSelectionReport) {
        if let Some(existing) = self
            .output
            .variant_selections
            .iter_mut()
            .find(|existing| existing.equivalence_group == report.equivalence_group)
        {
            *existing = report.clone();
        } else {
            self.output.variant_selections.push(report.clone());
        }
        self.selection_updates.push(report);
    }

    pub fn actual_selection(
        &mut self,
        group: &str,
        selected: Option<(&str, TextVariantKind)>,
        fallback: bool,
        rejected: &[(String, TextVariantKind)],
    ) {
        let mut report = self
            .output
            .variant_selections
            .iter()
            .find(|report| report.equivalence_group == group)
            .cloned()
            .unwrap_or_else(|| TextVariantSelectionReport {
                backend: VariantSelectionBackend::NativeSkia,
                equivalence_group: group.into(),
                selected_variant_id: None,
                selected_variant_kind: None,
                selected_reason: VariantSelectedReason::NoSupportedVariant,
                fallback_required: true,
                rejected_variants: Vec::new(),
            });
        let selected =
            selected.or_else(|| fallback.then_some(("textRun", TextVariantKind::TextRun)));
        report.selected_variant_id = selected.map(|(id, _)| id.into());
        report.selected_variant_kind = selected.map(|(_, kind)| kind);
        report.selected_reason = match report.selected_variant_kind {
            Some(TextVariantKind::GlyphRun) => VariantSelectedReason::GlyphRunStrictEligible,
            Some(TextVariantKind::GlyphOutline) => VariantSelectedReason::GlyphOutlineStrictProfile,
            Some(TextVariantKind::TextRun) => VariantSelectedReason::DefaultTextRunFallback,
            _ => VariantSelectedReason::NoSupportedVariant,
        };
        report.fallback_required = !matches!(
            report.selected_variant_kind,
            Some(TextVariantKind::GlyphRun | TextVariantKind::GlyphOutline)
        );
        report
            .rejected_variants
            .retain(|candidate| Some(candidate.variant_id.as_str()) != selected.map(|(id, _)| id));
        for (id, kind) in rejected {
            if !report
                .rejected_variants
                .iter()
                .any(|candidate| candidate.variant_id == *id)
            {
                // Actual native preparation includes shared contract checks,
                // resource construction and budget checks. Do not report a
                // rejected resource as selected based on contract checks alone.
                report.rejected_variants.push(TextVariantRejectReport {
                    variant_id: id.clone(),
                    variant_kind: *kind,
                    reasons: vec![VariantRejectReason::BackendDoesNotSupportVariant],
                });
            }
        }
        self.apply_selection(report);
    }

    pub fn finish_caches(
        &mut self,
        before: NativeSkiaCacheStats,
        after: NativeSkiaCacheStats,
        image_before: ImageReplayCounters,
        image_after: ImageReplayCounters,
    ) {
        let delta = |after: u64, before: u64| -> usize {
            usize::try_from(after.saturating_sub(before)).unwrap_or(usize::MAX)
        };
        let output = &mut self.output;
        output.static_picture_cache_hits = delta(after.pictures.hits, before.pictures.hits);
        output.static_picture_cache_misses = delta(after.pictures.misses, before.pictures.misses);
        output.static_picture_cache_evictions =
            delta(after.pictures.evictions, before.pictures.evictions);
        output.static_picture_cache_skipped_oversized = delta(
            after.pictures.skipped_oversized,
            before.pictures.skipped_oversized,
        );
        output.static_picture_cache_approx_bytes = after.pictures.approximate_bytes;
        // Content fingerprints are part of the key, so replacement bytes are
        // cache misses rather than a separate post-lookup mismatch event.
        output.image_effect_cache_hits = delta(
            image_after.binary_cache.hits,
            image_before.binary_cache.hits,
        );
        output.image_effect_cache_misses = delta(
            image_after.binary_cache.misses,
            image_before.binary_cache.misses,
        );
        output.image_effect_cache_evictions = delta(
            image_after.binary_cache.evictions,
            image_before.binary_cache.evictions,
        );
        output.image_effect_cache_skipped_oversized = delta(
            image_after.binary_cache.skipped_oversized,
            image_before.binary_cache.skipped_oversized,
        );
        output.image_effect_cache_approx_bytes = image_after.binary_cache.approximate_bytes;
        output.image_effect_preprocessed_bytes = image_after
            .preprocessed_bytes
            .saturating_sub(image_before.preprocessed_bytes);
        output.image_effect_preprocess_failures = image_after
            .preprocessing_failures
            .saturating_sub(image_before.preprocessing_failures);
        output.image_effect_fallback_to_filter = image_after
            .fallback_to_filter
            .saturating_sub(image_before.fallback_to_filter);
        output.tile_fallback_cap_hits = image_after
            .tile_cap_hits
            .saturating_sub(image_before.tile_cap_hits);
    }
}
