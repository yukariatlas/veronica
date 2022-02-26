use crate::{dataview::view, strategy::bollinger_band};

pub fn draw_bollinger_band_diagram(path: &str) {
    let data = std::fs::read_to_string(path).expect("Unable to read file");
    let views: Vec<view::BollingerBandView> = serde_yaml::from_str(&data).expect("Unable to parse yaml");
    let mut date_series = Vec::new();
    let mut open_series = Vec::new();
    let mut high_series = Vec::new();
    let mut low_series = Vec::new();
    let mut close_series = Vec::new();
    let mut sma_series = Vec::new();
    let mut upper_band_series = Vec::new();
    let mut lower_band_series = Vec::new();
    let mut plot = plotly::Plot::new();

    for view in views {
        date_series.push(view.date.format("%Y-%m-%d").to_string());
        open_series.push(view.open);
        high_series.push(view.high);
        low_series.push(view.low);
        close_series.push(view.close);
        sma_series.push(view.sma);
        upper_band_series.push(view.sma + bollinger_band::BAND_SIZE as f64 * view.sd);
        lower_band_series.push(view.sma - bollinger_band::BAND_SIZE as f64 * view.sd);
    }

    let trace_1 = plotly::Candlestick::new(date_series.clone(),
        open_series.clone(), high_series.clone(), low_series.clone(), close_series.clone())
        .name("Candlestick");
    let trace_2 = plotly::Scatter::new(date_series.clone(), sma_series.clone())
        .mode(plotly::common::Mode::Lines)
        .name("20 Period SMA");
    let trace_3 = plotly::Scatter::new(date_series.clone(), upper_band_series.clone())
        .mode(plotly::common::Mode::Lines)
        .name("Upper Band");
    let trace_4 = plotly::Scatter::new(date_series.clone(), lower_band_series.clone())
        .mode(plotly::common::Mode::Lines)
        .name("Lower Band");
    
    plot.add_trace(trace_1);
    plot.add_trace(trace_2);
    plot.add_trace(trace_3);
    plot.add_trace(trace_4);
    plot.show();
}