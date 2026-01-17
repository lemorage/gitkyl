//! Blame display component with commit attribution per line

use maud::{Markup, PreEscaped, html};

use crate::avatar::render as render_avatar;
use crate::git::BlameLine;
use crate::util::format_timestamp;

/// Computes recency score (0.0 = oldest, 1.0 = newest) for age stripe coloring
fn compute_recency(blame_lines: &[BlameLine]) -> Vec<f64> {
    if blame_lines.is_empty() {
        return vec![];
    }

    let timestamps: Vec<i64> = blame_lines.iter().map(|b| b.timestamp).collect();
    let min_ts = *timestamps.iter().min().unwrap_or(&0);
    let max_ts = *timestamps.iter().max().unwrap_or(&0);
    let range = (max_ts - min_ts) as f64;

    if range == 0.0 {
        return vec![1.0; blame_lines.len()];
    }

    timestamps
        .iter()
        .map(|ts| (*ts - min_ts) as f64 / range)
        .collect()
}

/// Converts recency score to HSL hue (30 = orange/recent, 220 = blue-gray/old)
fn recency_to_hue(recency: f64) -> u32 {
    let hue = 220.0 - (recency * 190.0); // 220 (old) -> 30 (new)
    hue as u32
}

/// Computes group index for alternating background colors
fn compute_group_indices(blame_lines: &[BlameLine]) -> Vec<usize> {
    let mut indices = Vec::with_capacity(blame_lines.len());
    let mut group_idx = 0;

    for (idx, blame) in blame_lines.iter().enumerate() {
        if idx > 0 && blame.commit_id != blame_lines[idx - 1].commit_id {
            group_idx += 1;
        }
        indices.push(group_idx);
    }

    indices
}

/// Renders blame table with commit info
pub fn blame_table(
    blame_lines: &[BlameLine],
    highlighted_lines: &[String],
    ref_name: &str,
    depth: usize,
) -> Markup {
    let commits_path = format!("{}commits/{}/page-1.html", "../".repeat(depth), ref_name);
    let group_indices = compute_group_indices(blame_lines);
    let recency_scores = compute_recency(blame_lines);

    html! {
        div class="blame-wrapper" {
            table class="blame-table" {
                tbody id="blame-code" {
                    @for (idx, (blame, highlighted)) in blame_lines.iter().zip(highlighted_lines.iter()).enumerate() {
                        @let is_group_start = idx == 0 || blame.commit_id != blame_lines[idx - 1].commit_id;
                        @let is_group_end = idx == blame_lines.len() - 1 || blame.commit_id != blame_lines[idx + 1].commit_id;
                        @let group_parity = if group_indices[idx].is_multiple_of(2) { "group-even" } else { "group-odd" };
                        @let hue = recency_to_hue(recency_scores[idx]);
                        tr id=(format!("L{}", blame.line_num))
                           class={"blame-line " (group_parity) @if is_group_start { " group-start" } @if is_group_end { " group-end" }}
                           data-commit=(blame.commit_id) {
                            td class="blame-age" style=(format!("--age-hue: {}", hue)) {
                                span class="age-stripe" {}
                            }
                            td class="blame-info" {
                                @if is_group_start {
                                    (render_avatar(&blame.author, 16))
                                    a href=(commits_path)
                                      class="blame-hash" {
                                        (blame.short_id)
                                    }
                                    div class="blame-tooltip" {
                                        div class="tooltip-row" {
                                            span class="tooltip-author" { (&blame.author) }
                                            span class="tooltip-sep" { " · " }
                                            span class="tooltip-time" { (format_timestamp(blame.timestamp)) }
                                        }
                                        div class="tooltip-summary" { (&blame.summary) }
                                    }
                                }
                            }
                            td class="line-number" data-line=(blame.line_num) {
                                a href=(format!("#L{}", blame.line_num)) { (blame.line_num) }
                            }
                            td class="line-content" {
                                (PreEscaped(highlighted))
                            }
                        }
                    }
                }
            }
        }
    }
}
