use moviebox_tui::cache::md5_hex;
use moviebox_tui::providers::tv::parser::M3UParser;
use moviebox_tui::tui::app::App;
use moviebox_tui::tui::text::truncate_width;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::time::Instant;

fn baseline_truncate_width(value: &str, max_width: usize) -> String {
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;
    if UnicodeWidthStr::width(value) <= max_width {
        return value.to_string();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let content_width = max_width - 3;
    let mut output = String::new();
    let mut used = 0;
    for grapheme in value.graphemes(true) {
        let grapheme_width = UnicodeWidthStr::width(grapheme);
        if used + grapheme_width > content_width {
            break;
        }
        output.push_str(grapheme);
        used += grapheme_width;
    }
    output.push_str("...");
    output
}

fn baseline_md5_hex(value: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(value.as_bytes());
    let result = hasher.finalize();
    let mut safe_query = String::with_capacity(32);
    for b in result {
        use std::fmt::Write;
        let _ = write!(&mut safe_query, "{:02x}", b);
    }
    safe_query
}

#[test]
fn test_benchmark_performance_improvements_matrix() {
    const ITERATIONS: usize = 10_000;

    let sample_titles = [
        "Breaking Bad",
        "Inception (2010)",
        "The Lord of the Rings: The Fellowship of the Ring",
        "Interstellar [1080p Web-DL Multi Sub]",
        "Game of Thrones: Season 1 Episode 1 - Winter Is Coming",
        "Stranger Things",
        "Oppenheimer (2023)",
        "Avengers: Endgame",
    ];

    let mut baseline_truncate_duration = std::time::Duration::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        for i in 0..ITERATIONS {
            let title = sample_titles[i % sample_titles.len()];
            let _ = baseline_truncate_width(title, 30);
        }
        baseline_truncate_duration = baseline_truncate_duration.min(t.elapsed());
    }

    let mut opt_truncate_duration = std::time::Duration::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        for i in 0..ITERATIONS {
            let title = sample_titles[i % sample_titles.len()];
            let _ = truncate_width(title, 30);
        }
        opt_truncate_duration = opt_truncate_duration.min(t.elapsed());
    }

    println!(
        "BENCHMARK: truncate_width ({} ops)\n  Baseline:  {:?}\n  Optimized: {:?}\n  Speedup:   {:.2}x",
        ITERATIONS,
        baseline_truncate_duration,
        opt_truncate_duration,
        baseline_truncate_duration.as_nanos() as f64
            / opt_truncate_duration.as_nanos().max(1) as f64
    );
    if cfg!(debug_assertions) {
        assert!(opt_truncate_duration <= baseline_truncate_duration * 2);
    } else {
        assert!(opt_truncate_duration <= baseline_truncate_duration);
    }

    let sample_urls = [
        "https://moviebox.ph/api/subject/1001?season=1&episode=1",
        "https://moviebox.ph/api/subject/1002?season=1&episode=2",
        "https://moviebox.ph/api/subject/1003?season=1&episode=3",
        "https://moviebox.ph/api/subject/1004?season=1&episode=4",
    ];

    let mut baseline_md5_duration = std::time::Duration::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        for i in 0..ITERATIONS {
            let url = sample_urls[i % sample_urls.len()];
            let _ = baseline_md5_hex(url);
        }
        baseline_md5_duration = baseline_md5_duration.min(t.elapsed());
    }

    let mut opt_md5_duration = std::time::Duration::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        for i in 0..ITERATIONS {
            let url = sample_urls[i % sample_urls.len()];
            let _ = md5_hex(url);
        }
        opt_md5_duration = opt_md5_duration.min(t.elapsed());
    }

    println!(
        "BENCHMARK: md5_hex ({} ops)\n  Baseline:  {:?}\n  Optimized: {:?}\n  Speedup:   {:.2}x",
        ITERATIONS,
        baseline_md5_duration,
        opt_md5_duration,
        baseline_md5_duration.as_nanos() as f64 / opt_md5_duration.as_nanos().max(1) as f64
    );
    if cfg!(debug_assertions) {
        assert!(opt_md5_duration <= baseline_md5_duration * 2);
    } else {
        assert!(opt_md5_duration <= baseline_md5_duration);
    }

    let mut playlist_data = String::from("#EXTM3U\n");
    for i in 0..500 {
        playlist_data.push_str(&format!(
            "#EXTINF:-1 tvg-id=\"channel.{i}\" tvg-logo=\"http://logo.png/{i}.png\" group-title=\"Entertainment\",Channel {i}\nhttp://stream.example.com/{i}.m3u8\n"
        ));
    }

    let parser = M3UParser::new();
    let t4 = Instant::now();
    for _ in 0..100 {
        let channels = parser.parse_m3u(&playlist_data);
        assert_eq!(channels.len(), 500);
    }
    let m3u_duration = t4.elapsed();
    println!(
        "BENCHMARK: parse_m3u 500 channels x 100 runs\n  Duration: {:?} ({:.2} µs/run)",
        m3u_duration,
        m3u_duration.as_micros() as f64 / 100.0
    );

    let dims = [(80, 24), (120, 30), (160, 40)];
    for (w, h) in dims {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        let t_draw = Instant::now();
        const DRAW_ITERATIONS: usize = 1_000;
        for _ in 0..DRAW_ITERATIONS {
            terminal.draw(|f| app.draw(f)).unwrap();
        }
        let draw_elapsed = t_draw.elapsed();
        let avg_us = draw_elapsed.as_micros() as f64 / DRAW_ITERATIONS as f64;
        println!(
            "BENCHMARK: TUI draw ({}x{}) x {} frames\n  Total: {:?}\n  Average: {:.2} µs/frame",
            w, h, DRAW_ITERATIONS, draw_elapsed, avg_us
        );
    }
}
