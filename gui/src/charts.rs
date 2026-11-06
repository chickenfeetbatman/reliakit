//! Charts rendered with plotters into in-memory bitmaps, handed to
//! Slint as images. Rendered at 2x the display size so they stay
//! crisp. Chart text uses the bundled Inter font, registered with
//! plotters at startup.

use plotters::coord::Shift;
use plotters::prelude::*;
use reliakit::degradation;
use reliakit::distributions as dist_mod;
use reliakit::growth;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

pub const CHART_W: u32 = 1280;
pub const CHART_H: u32 = 600;

const BG: RGBColor = RGBColor(0x21, 0x23, 0x26);
const GRID: RGBColor = RGBColor(0x3A, 0x3C, 0x40);
const TEXT: RGBColor = RGBColor(0xF2, 0xF2, 0xF7);
const MUTED: RGBColor = RGBColor(0xA7, 0xA7, 0xAD);
const TINT: RGBColor = RGBColor(0x0A, 0x84, 0xFF);
const S2: RGBColor = RGBColor(0x64, 0xD2, 0xFF);
const S3: RGBColor = RGBColor(0xBF, 0x5A, 0xF2);
const S4: RGBColor = RGBColor(0xFF, 0x64, 0x82);
const S5: RGBColor = RGBColor(0x30, 0xD1, 0x58);
pub const SERIES: [RGBColor; 5] = [TINT, S2, S3, S4, S5];

static INTER_REGULAR: &[u8] = include_bytes!("../ui/fonts/Inter-Regular.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../ui/fonts/Inter-SemiBold.ttf");

pub fn init_fonts() {
    let _ = plotters::style::register_font("sans-serif", FontStyle::Normal, INTER_REGULAR);
    let _ = plotters::style::register_font("sans-serif", FontStyle::Bold, INTER_SEMIBOLD);
}

fn to_image(buffer: Vec<u8>, w: u32, h: u32) -> Image {
    // Plotters' default bitmap pixel format is RGB, 3 bytes per
    // pixel in R, G, B order; convert to RGBA for Slint.
    let mut px = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    let out = px.make_mut_slice();
    for (i, o) in out.iter_mut().enumerate() {
        let r = buffer[i * 3];
        let g = buffer[i * 3 + 1];
        let b = buffer[i * 3 + 2];
        *o = Rgba8Pixel { r, g, b, a: 255 };
    }
    Image::from_rgba8(px)
}

fn canvas<F>(draw: F) -> Option<Image>
where
    F: FnOnce(&DrawingArea<BitMapBackend, Shift>) -> Result<(), Box<dyn std::error::Error>>,
{
    let mut buffer = vec![0u8; (CHART_W * CHART_H * 3) as usize];
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (CHART_W, CHART_H)).into_drawing_area();
        root.fill(&BG).ok()?;
        draw(&root).ok()?;
        root.present().ok()?;
    }
    Some(to_image(buffer, CHART_W, CHART_H))
}

fn label_style(size: i32) -> TextStyle<'static> {
    ("sans-serif", size).into_font().color(&MUTED)
}
fn title_style(size: i32) -> TextStyle<'static> {
    ("sans-serif", size).into_font().color(&TEXT)
}

/// Reliability curve of a fit, or the probability plot points when
/// `prob` is set (empirical F against fitted F with a y=x reference).
pub fn life_chart(fit: &dist_mod::FitResult, prob: bool) -> Option<Image> {
    if prob {
        let (_times, f_emp, f_theo) = dist_mod::probability_plot_data(fit);
        let pts: Vec<(f64, f64)> = f_emp.iter().zip(&f_theo).map(|(a, b)| (*a, *b)).collect();
        return canvas(|root| {
            let mut chart = ChartBuilder::on(root)
                .caption("Probability plot", title_style(26))
                .margin(18)
                .x_label_area_size(46)
                .y_label_area_size(56)
                .build_cartesian_2d(0.0f64..1.0f64, 0.0f64..1.0f64)?;
            chart.configure_mesh()
                .x_desc("Empirical F (median rank)").y_desc("Fitted F")
                .x_label_style(label_style(19)).y_label_style(label_style(19))
                .axis_desc_style(label_style(20))
                .light_line_style(GRID.mix(0.5)).bold_line_style(GRID)
                .axis_style(MUTED).draw()?;
            chart.draw_series(LineSeries::new(vec![(0.0, 0.0), (1.0, 1.0)], MUTED.mix(0.7)))?
                .label("y = x").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 22, y)], MUTED));
            chart.draw_series(PointSeries::of_element(pts, 6, TINT.filled(), &|c, s, st| {
                EmptyElement::at(c) + Circle::new((0, 0), s, st)
            }))?.label("Failures").legend(|(x, y)| Circle::new((x + 11, y), 5, TINT.filled()));
            chart.configure_series_labels()
                .background_style(BG.mix(0.85)).border_style(GRID)
                .label_font(label_style(19)).draw()?;
            Ok(())
        });
    }
    let tmax = fit.times.iter().cloned().fold(1.0f64, f64::max) * 1.15;
    let pts: Vec<(f64, f64)> = (0..=120).map(|i| {
        let t = tmax * i as f64 / 120.0;
        (t, fit.reliability(t))
    }).collect();
    let name = dist_mod::dist_label(&fit.dist).to_string();
    canvas(move |root| {
        let mut chart = ChartBuilder::on(root)
            .caption("Reliability curve", title_style(26))
            .margin(18)
            .x_label_area_size(46)
            .y_label_area_size(56)
            .build_cartesian_2d(0.0f64..tmax, 0.0f64..1.02f64)?;
        chart.configure_mesh()
            .x_desc("Time").y_desc("Reliability R(t)")
            .x_label_style(label_style(19)).y_label_style(label_style(19))
            .axis_desc_style(label_style(20))
            .light_line_style(GRID.mix(0.5)).bold_line_style(GRID)
            .axis_style(MUTED).draw()?;
        chart.draw_series(LineSeries::new(pts, TINT.stroke_width(3)))?
            .label(name).legend(|(x, y)| PathElement::new(vec![(x, y), (x + 22, y)], TINT.stroke_width(3)));
        chart.configure_series_labels()
            .background_style(BG.mix(0.85)).border_style(GRID)
            .label_font(label_style(19)).draw()?;
        Ok(())
    })
}

/// Degradation paths: raw points and fitted path lines per unit,
/// with the failure threshold.
pub fn degradation_chart(
    units: &[(Vec<f64>, Vec<f64>)],
    result: &degradation::DegradationResult,
    model: &str,
    threshold: f64,
) -> Option<Image> {
    let mut tmax = 1.0f64;
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;
    for (t, y) in units {
        for v in t { tmax = tmax.max(*v); }
        for v in y { ymin = ymin.min(*v); ymax = ymax.max(*v); }
    }
    if !ymin.is_finite() { return None; }
    let pad = (ymax - ymin).abs() * 0.12 + 1.0;
    let (lo, hi) = (ymin.min(threshold) - pad, ymax.max(threshold) + pad);
    let units_owned: Vec<(Vec<f64>, Vec<f64>)> = units.to_vec();
    let paths = result.paths.clone();
    let model = model.to_string();
    canvas(move |root| {
        let mut chart = ChartBuilder::on(root)
            .caption("Degradation paths", title_style(26))
            .margin(18)
            .x_label_area_size(46)
            .y_label_area_size(60)
            .build_cartesian_2d(0.0f64..tmax * 1.1, lo..hi)?;
        chart.configure_mesh()
            .x_desc("Time").y_desc("Measurement")
            .x_label_style(label_style(19)).y_label_style(label_style(19))
            .axis_desc_style(label_style(20))
            .light_line_style(GRID.mix(0.5)).bold_line_style(GRID)
            .axis_style(MUTED).draw()?;
        chart.draw_series(LineSeries::new(vec![(0.0, threshold), (tmax * 1.1, threshold)], S4.stroke_width(2)))?
            .label("Threshold").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 22, y)], S4));
        for (i, (t, y)) in units_owned.iter().enumerate() {
            let col = SERIES[i % SERIES.len()];
            let raw: Vec<(f64, f64)> = t.iter().zip(y).map(|(a, b)| (*a, *b)).collect();
            chart.draw_series(PointSeries::of_element(raw, 5, col.filled(), &|c, s, st| {
                EmptyElement::at(c) + Circle::new((0, 0), s, st)
            }))?.label(format!("Unit {} data", i + 1)).legend(move |(x, y)| Circle::new((x + 11, y), 5, col.filled()));
            if let Some((a, b)) = paths.get(i) {
                let tt = t.iter().cloned().fold(1.0f64, f64::max) * 1.2;
                let line: Vec<(f64, f64)> = (0..=40).map(|k| {
                    let x = tt * k as f64 / 40.0;
                    (x, degradation::path_value(*a, *b, x, &model))
                }).collect();
                chart.draw_series(LineSeries::new(line, col.stroke_width(2)))?;
            }
        }
        chart.configure_series_labels()
            .background_style(BG.mix(0.85)).border_style(GRID)
            .label_font(label_style(19)).draw()?;
        Ok(())
    })
}

/// Crow-AMSAA expected cumulative failures against observed steps.
pub fn growth_chart(fit: &growth::CrowAmsaaFit, times: &[f64]) -> Option<Image> {
    let mut sorted = times.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let obs: Vec<(f64, f64)> = sorted.iter().enumerate().map(|(i, t)| (*t, (i + 1) as f64)).collect();
    let tmax = fit.t_end.max(1.0);
    let ymax = (obs.len() as f64).max(fit.lam * tmax.powf(fit.beta)) * 1.1 + 1.0;
    let (lam, beta) = (fit.lam, fit.beta);
    canvas(move |root| {
        let mut chart = ChartBuilder::on(root)
            .caption("Reliability growth", title_style(26))
            .margin(18)
            .x_label_area_size(46)
            .y_label_area_size(56)
            .build_cartesian_2d(0.0f64..tmax, 0.0f64..ymax)?;
        chart.configure_mesh()
            .x_desc("Cumulative test time").y_desc("Cumulative failures")
            .x_label_style(label_style(19)).y_label_style(label_style(19))
            .axis_desc_style(label_style(20))
            .light_line_style(GRID.mix(0.5)).bold_line_style(GRID)
            .axis_style(MUTED).draw()?;
        let expected: Vec<(f64, f64)> = (0..=60).map(|k| {
            let t = tmax * k as f64 / 60.0;
            (t, lam * t.powf(beta))
        }).collect();
        chart.draw_series(LineSeries::new(expected, TINT.stroke_width(3)))?
            .label("Crow-AMSAA expected").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 22, y)], TINT.stroke_width(3)));
        chart.draw_series(PointSeries::of_element(obs, 6, S2.filled(), &|c, s, st| {
            EmptyElement::at(c) + Circle::new((0, 0), s, st)
        }))?.label("Observed failures").legend(|(x, y)| Circle::new((x + 11, y), 5, S2.filled()));
        chart.configure_series_labels()
            .background_style(BG.mix(0.85)).border_style(GRID)
            .label_font(label_style(19)).draw()?;
        Ok(())
    })
}

/// Reliability at use stress over time from an ALT fit.
pub fn alt_chart(series: &[(f64, f64)]) -> Option<Image> {
    if series.is_empty() {
        return None;
    }
    let tmax = series.last().map(|p| p.0).unwrap_or(1.0).max(1.0);
    let pts: Vec<(f64, f64)> = series.to_vec();
    canvas(move |root| {
        let mut chart = ChartBuilder::on(root)
            .caption("Reliability at use stress", title_style(26))
            .margin(18)
            .x_label_area_size(46)
            .y_label_area_size(56)
            .build_cartesian_2d(0.0f64..tmax, 0.0f64..1.02f64)?;
        chart.configure_mesh()
            .x_desc("Time").y_desc("Reliability R(t) at use stress")
            .x_label_style(label_style(19)).y_label_style(label_style(19))
            .axis_desc_style(label_style(20))
            .light_line_style(GRID.mix(0.5)).bold_line_style(GRID)
            .axis_style(MUTED).draw()?;
        chart.draw_series(LineSeries::new(pts, TINT.stroke_width(3)))?
            .label("Least squares fit").legend(|(x, y)| PathElement::new(vec![(x, y), (x + 22, y)], TINT.stroke_width(3)));
        chart.configure_series_labels()
            .background_style(BG.mix(0.85)).border_style(GRID)
            .label_font(label_style(19)).draw()?;
        Ok(())
    })
}

/// Markov transient state probabilities.
pub fn markov_chart(series: &[(String, Vec<(f64, f64)>)]) -> Option<Image> {
    if series.is_empty() {
        return None;
    }
    let tmax = series.iter().flat_map(|(_, pts)| pts.iter().map(|p| p.0)).fold(1.0f64, f64::max);
    let owned: Vec<(String, Vec<(f64, f64)>)> = series.to_vec();
    canvas(move |root| {
        let mut chart = ChartBuilder::on(root)
            .caption("Transient state probabilities", title_style(26))
            .margin(18)
            .x_label_area_size(46)
            .y_label_area_size(56)
            .build_cartesian_2d(0.0f64..tmax, 0.0f64..1.02f64)?;
        chart.configure_mesh()
            .x_desc("Time").y_desc("Probability")
            .x_label_style(label_style(19)).y_label_style(label_style(19))
            .axis_desc_style(label_style(20))
            .light_line_style(GRID.mix(0.5)).bold_line_style(GRID)
            .axis_style(MUTED).draw()?;
        for (i, (name, pts)) in owned.iter().enumerate() {
            let col = SERIES[i % SERIES.len()];
            chart.draw_series(LineSeries::new(pts.clone(), col.stroke_width(3)))?
                .label(name.clone()).legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 22, y)], col.stroke_width(3)));
        }
        chart.configure_series_labels()
            .background_style(BG.mix(0.85)).border_style(GRID)
            .label_font(label_style(19)).draw()?;
        Ok(())
    })
}
