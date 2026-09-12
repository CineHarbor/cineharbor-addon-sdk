//! HLS 广告分段过滤（对齐 web `src/lib/ad-filter.ts`）。
//!
//! CMS 源常通过 `#EXT-X-DISCONTINUITY` 拼接广告组。保留最长组作为主内容，
//! 其余短组（按时长/段数）和广告域名分段删除。

#[derive(Debug, Clone)]
pub struct AdFilterConfig {
    pub enabled: bool,
    pub min_ad_duration: f64,
    pub max_ad_duration: f64,
    pub max_consecutive_ad_segments: usize,
}

impl Default for AdFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_ad_duration: 3.0,
            max_ad_duration: 120.0,
            max_consecutive_ad_segments: 15,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterResult {
    pub filtered: String,
    pub ads_removed: usize,
    pub ads_duration: f64,
    pub changed: bool,
}

const FORCE_AD_DOMAIN_PATTERNS: &[&str] = &[
    "ffzyad",
    "vip.ffzyad.com",
    "bytegoofy.com",
    "mimg.0c1q0l.cn",
    "mc.usihnbcq.cn",
    "wan.51img1.com",
    "iqiyi.hbuioo.com",
    "casino",
    "macau",
    "aomen",
    "gambling",
    "bet365",
    "1xbet",
    "188bet",
    "22bet",
    "bookmaker",
    "sportsbook",
];

const SAFE_DOMAINS: &[&str] = &[
    "hhuus.com",
    "play-cdn",
    "alicdn.com",
    "aliyuncs.com",
    "myqcloud.com",
    "bcebos.com",
];

const AD_DOMAIN_PATTERNS: &[&str] = &[
    "doubleclick",
    "googlesyndication",
    "adsystem",
    "adservice",
    "/ad/",
    "/ads/",
    "preroll",
    "midroll",
    "postroll",
];

#[derive(Clone)]
struct ParsedSegment {
    duration: f64,
    discontinuity_group: usize,
    line_index: usize,
    url_line_index: usize,
    is_ad_domain: bool,
}

fn is_ad_domain(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    if FORCE_AD_DOMAIN_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
    {
        return true;
    }
    if SAFE_DOMAINS.iter().any(|safe| lower.contains(safe)) {
        return false;
    }
    AD_DOMAIN_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

fn parse_extinf_duration(line: &str) -> f64 {
    line.strip_prefix("#EXTINF:")
        .and_then(|rest| {
            rest.split([',', ':'])
                .next()
                .and_then(|value| value.parse::<f64>().ok())
        })
        .unwrap_or(0.0)
}

pub fn filter_m3u8(content: &str, config: &AdFilterConfig) -> FilterResult {
    if !config.enabled || content.contains("#EXT-X-STREAM-INF") {
        return FilterResult {
            filtered: content.to_string(),
            ads_removed: 0,
            ads_duration: 0.0,
            changed: false,
        };
    }

    let lines: Vec<&str> = content.split('\n').map(str::trim).collect();
    let mut segments = Vec::new();
    let mut current_inf: Option<(f64, usize)> = None;
    let mut discontinuity_count = 0usize;
    let mut current_group = 0usize;

    for (index, line) in lines.iter().enumerate() {
        if line.starts_with("#EXT-X-DISCONTINUITY") {
            discontinuity_count += 1;
            current_group = discontinuity_count;
            continue;
        }
        if line.starts_with("#EXTINF:") {
            current_inf = Some((parse_extinf_duration(line), index));
            continue;
        }
        if let Some((duration, line_index)) = current_inf.take() {
            if !line.is_empty() && !line.starts_with('#') {
                segments.push(ParsedSegment {
                    duration,
                    discontinuity_group: current_group,
                    line_index,
                    url_line_index: index,
                    is_ad_domain: is_ad_domain(line),
                });
            }
        }
    }

    if discontinuity_count == 0 && segments.iter().all(|segment| !segment.is_ad_domain) {
        return FilterResult {
            filtered: content.to_string(),
            ads_removed: 0,
            ads_duration: 0.0,
            changed: false,
        };
    }

    let mut ad_indices = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        if segment.is_ad_domain {
            ad_indices.push(index);
        }
    }

    let mut group_keys: Vec<usize> = segments
        .iter()
        .map(|segment| segment.discontinuity_group)
        .collect();
    group_keys.sort_unstable();
    group_keys.dedup();

    if group_keys.len() > 1 {
        let mut main_group = group_keys[0];
        let mut max_duration = 0.0;
        for group in &group_keys {
            let duration: f64 = segments
                .iter()
                .filter(|segment| segment.discontinuity_group == *group)
                .map(|segment| segment.duration)
                .sum();
            if duration > max_duration {
                max_duration = duration;
                main_group = *group;
            }
        }

        for group in &group_keys {
            if *group == main_group {
                continue;
            }
            let group_segments: Vec<usize> = segments
                .iter()
                .enumerate()
                .filter(|(_, segment)| segment.discontinuity_group == *group)
                .map(|(index, _)| index)
                .collect();
            let group_duration: f64 = group_segments
                .iter()
                .map(|index| segments[*index].duration)
                .sum();
            if group_duration > config.max_ad_duration {
                continue;
            }
            let is_ad_by_duration = group_duration >= config.min_ad_duration
                && group_duration <= config.max_ad_duration;
            let is_ad_by_count = group_segments.len() <= config.max_consecutive_ad_segments;
            if is_ad_by_duration && is_ad_by_count {
                ad_indices.extend(group_segments);
            }
        }
    }

    ad_indices.sort_unstable();
    ad_indices.dedup();
    if ad_indices.is_empty() {
        return FilterResult {
            filtered: content.to_string(),
            ads_removed: 0,
            ads_duration: 0.0,
            changed: false,
        };
    }

    let ads_duration: f64 = ad_indices
        .iter()
        .map(|index| segments[*index].duration)
        .sum();
    let lines_to_remove: std::collections::HashSet<usize> = ad_indices
        .iter()
        .flat_map(|index| [segments[*index].line_index, segments[*index].url_line_index])
        .collect();

    let filtered_lines: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            if lines_to_remove.contains(&index) {
                return None;
            }
            if line.starts_with("#EXT-X-DISCONTINUITY") {
                return Some(*line);
            }
            Some(*line)
        })
        .collect();

    let mut cleaned = Vec::new();
    for (index, line) in filtered_lines.iter().enumerate() {
        if line.starts_with("#EXT-X-DISCONTINUITY") {
            let next_non_empty = filtered_lines[index + 1..]
                .iter()
                .find(|value| !value.is_empty())
                .copied()
                .unwrap_or("");
            if next_non_empty.starts_with("#EXT-X-DISCONTINUITY")
                || next_non_empty.starts_with("#EXT-X-ENDLIST")
                || next_non_empty.is_empty()
            {
                continue;
            }
        }
        cleaned.push(*line);
    }

    let mut final_lines = Vec::new();
    let mut found_first_segment = false;
    for line in cleaned {
        if !found_first_segment && line.starts_with("#EXT-X-DISCONTINUITY") {
            continue;
        }
        if line.starts_with("#EXTINF:") {
            found_first_segment = true;
        }
        final_lines.push(line);
    }

    let no_discontinuity: Vec<&str> = final_lines
        .into_iter()
        .filter(|line| !line.starts_with("#EXT-X-DISCONTINUITY"))
        .collect();

    FilterResult {
        filtered: no_discontinuity.join("\n"),
        ads_removed: ad_indices.len(),
        ads_duration,
        changed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_casino_ad_domain() {
        let playlist = [
            "#EXTM3U",
            "#EXT-X-VERSION:3",
            "#EXTINF:6.0,",
            "https://vip.ffzyad.com/casino-roll.ts",
            "#EXTINF:10.0,",
            "https://video.example.com/main.ts",
            "#EXT-X-ENDLIST",
        ]
        .join("\n");
        let result = filter_m3u8(&playlist, &AdFilterConfig::default());
        assert!(result.changed);
        assert_eq!(result.ads_removed, 1);
        assert!(!result.filtered.contains("vip.ffzyad.com"));
        assert!(result.filtered.contains("video.example.com/main.ts"));
    }
}
