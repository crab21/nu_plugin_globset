
use std::env;

use chrono::{Datelike, NaiveDate};
use csv::Reader;
use plotters::prelude::*;
use rand::Rng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <csv_file_path>", args[0]);
        std::process::exit(1);
    }
    let csv_path = &args[1];
    // 1️⃣ 读取 CSV 文件
    let mut rdr = Reader::from_path(&csv_path)?;
    let mut data: Vec<(NaiveDate, f64)> = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let date = NaiveDate::parse_from_str(&record[0], "%Y%m%d")?;
        let usage: f64 = record[1].parse()?;
        data.push((date, usage));
    }

    // 2️⃣ 按日期排序
    data.sort_by_key(|(d, _)| *d);

    // 3️⃣ 获取日期范围与最大值
    let min_date = *data.iter().map(|(d, _)| d).min().unwrap();
    let max_date = *data.iter().map(|(d, _)| d).max().unwrap();
    let max_usage = data.iter().map(|(_, u)| *u).fold(0.0, f64::max);
    print!("{}", data.len());
    // 4️⃣ 创建绘图区域
    let root = BitMapBackend::new("usage.png", (7680, 4320)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Daily Usage", ("sans-serif", 100))
        .margin(15)
        .x_label_area_size(20)
        .y_label_area_size(20)
        .build_cartesian_2d(
            min_date..max_date.succ_opt().unwrap().succ_opt().unwrap(),
            0f64..max_usage * 1.2,
        )?;
    chart.plotting_area().fill(&RGBColor(250, 250, 250))?;

    let x_ticks: Vec<NaiveDate> = data
        .iter()
        .map(|(d, _)| *d)
        .filter(|d| d.weekday().num_days_from_monday() == 0) // 星期一
        .chain(std::iter::once(data.last().unwrap().0)) // 保证最后一天显示
        .collect();

    chart
        .configure_mesh()
        .x_labels(x_ticks.len())
        .x_label_formatter(&|_d| {
                "".to_string()
        })
        .x_desc("Date")
        .y_desc("Usage")
        .axis_desc_style(("sans-serif", 50).into_font().color(&BLACK)) // 横纵轴标题字体
        .x_label_style(("sans-serif", 50).into_font().color(&BLACK)) // x_labels 字体大小和颜色
        .y_label_style(("sans-serif", 50).into_font().color(&BLACK)) // y_labels 字体
        .light_line_style(&RGBColor(220, 220, 220)) // 网格线浅灰
        .draw()?;

    // 5️⃣ 绘制折线
    chart.draw_series(LineSeries::new(data.iter().map(|(d, u)| (*d, *u)), &BLUE))?;

    let mut rng = rand::rng();

    for (date, usage) in &data {
        let color = if *usage > 10.0 { &RED } else { &BLACK };
        let jitter = rng.random_range(-max_usage * 0.01..max_usage * 0.01); // ±1% max_usage
        chart.draw_series(std::iter::once(Text::new(
            format!("{:.1}", usage),
            (*date, *usage + max_usage * 0.02 + jitter),
            ("sans-serif", 20).into_font().color(&color),
        )))?;
    }

    for (_, (date, _)) in data.iter().enumerate() {
      if x_ticks.contains(date) == false {
          continue;
      } 
      chart.draw_series(std::iter::once(
          Text::new(
              date.format("%m-%d").to_string(),
              (*date, 0.0), // Y 坐标放在轴下
              ("sans-serif", 30).into_font().color(&BLACK),
          )
      ))?;
    }

    root.present()?; // 将缓冲区写入 PNG 文件
    println!("✅ 已生成 usage.png");
    Ok(())
}
