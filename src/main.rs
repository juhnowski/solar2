use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Calendar, DrawingArea, Entry, Image, Label, Orientation,
    Scale, ScrolledWindow,
};
use spa::{StdFloatOps, solar_position, sunrise_and_set};
use std::cell::RefCell;
use std::rc::Rc;

struct AppState {
    lat: f64,
    lon: f64,
    date: NaiveDate,
    solar_file: String,
    helio_file: String,
    x_offset: i32,
    y_offset: i32,
    x_scale: f64,
    y_scale: f64,
}

fn format_time(minutes: f64) -> String {
    let h = (minutes / 60.0) as u32;
    let m = (minutes % 60.0) as u32;
    format!("{:02}:{:02}", h, m)
}

fn main() {
    let app = Application::builder()
        .application_id("com.solar.app.fixed_final")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let state = Rc::new(RefCell::new(AppState {
        lat: 56.502815,
        lon: 44.804089,
        date: Utc::now().date_naive(),
        solar_file: "/home/ilya/Pictures/solar/IMG_20251013_0003.jpg".to_string(),
        helio_file: "/home/ilya/Pictures/helio/IMG_20251013_0003.jpg".to_string(),
        x_offset: 0,
        y_offset: 0,
        x_scale: 1.0,
        y_scale: 1.0,
    }));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Solarography 2025")
        .default_width(1200)
        .default_height(800)
        .build();

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();
    window.set_child(Some(&scrolled_window));

    let main_box = Box::new(Orientation::Vertical, 10);
    // Исправлено: замена set_margin_all
    main_box.set_margin_start(10);
    main_box.set_margin_end(10);
    main_box.set_margin_top(10);
    main_box.set_margin_bottom(10);
    scrolled_window.set_child(Some(&main_box));

    let lat_entry = Entry::builder().text("56.502815").build();
    let lon_entry = Entry::builder().text("44.804089").build();
    main_box.append(&Label::new(Some("Координаты:")));
    main_box.append(&lat_entry);
    main_box.append(&lon_entry);

    let calendar = Calendar::new();
    main_box.append(&calendar);

    let time_slider = Scale::builder()
        .orientation(Orientation::Horizontal)
        .draw_value(true)
        .value_pos(gtk4::PositionType::Top)
        .digits(0)
        .build();

    time_slider.set_format_value_func(|_, val| format_time(val));
    main_box.append(&Label::new(Some("Локальное время (Рассвет — Закат)")));
    main_box.append(&time_slider);

    let update_slider = {
        let st = state.clone();
        let ts = time_slider.clone();
        move || {
            let s = st.borrow();
            let dt_base = Utc
                .with_ymd_and_hms(s.date.year(), s.date.month(), s.date.day(), 12, 0, 0)
                .unwrap();
            if let Ok(spa::SunriseAndSet::Daylight(sr, ss)) =
                sunrise_and_set::<StdFloatOps>(dt_base, s.lat, s.lon)
            {
                let start_m = sr.hour() as f64 * 60.0 + sr.minute() as f64;
                let end_m = ss.hour() as f64 * 60.0 + ss.minute() as f64;
                ts.set_range(start_m, end_m);
                ts.clear_marks();
                ts.add_mark(
                    start_m,
                    gtk4::PositionType::Bottom,
                    Some(&format!("Рассвет: {}", format_time(start_m))),
                );
                ts.add_mark(
                    end_m,
                    gtk4::PositionType::Bottom,
                    Some(&format!("Закат: {}", format_time(end_m))),
                );
            }
        }
    };

    let (box_solar, _ent_s, _img_s) =
        create_file_picker_row("Соларография", &state.borrow().solar_file);
    let (box_helio, _ent_h, _img_h) =
        create_file_picker_row("Гелиография", &state.borrow().helio_file);
    main_box.append(&box_solar);
    main_box.append(&box_helio);

    let drawing_area_1 = DrawingArea::new();
    let scroll_area_1 = ScrolledWindow::builder()
        .min_content_height(400)
        .child(&drawing_area_1)
        .build();
    main_box.append(&scroll_area_1);

    let scale_box = Box::new(Orientation::Horizontal, 5);
    let x_off_ent = Entry::builder().text("0").build();
    let x_scale_ent = Entry::builder().text("1.0").build();
    scale_box.append(&Label::new(Some("Сдвиг/Масштаб:")));
    scale_box.append(&x_off_ent);
    scale_box.append(&x_scale_ent);
    main_box.append(&scale_box);

    let drawing_area_2 = DrawingArea::new();
    main_box.append(&drawing_area_2);

    let update_canvas = {
        let st = state.clone();
        let da1 = drawing_area_1.clone();
        let da2 = drawing_area_2.clone();
        move || {
            let s = st.borrow();
            if let Ok(pb) = gdk_pixbuf::Pixbuf::from_file(&s.solar_file) {
                da1.set_content_width((pb.width() as f64 * s.x_scale).abs() as i32);
                da1.set_content_height((pb.height() as f64 * s.y_scale).abs() as i32);
            }
            da1.queue_draw();
            da2.queue_draw();
        }
    };

    let up_c = update_canvas.clone();
    let up_s = update_slider.clone();
    let st_cal = state.clone();
    calendar.connect_day_selected(move |c| {
        let d = c.date();
        if let Some(nd) =
            NaiveDate::from_ymd_opt(d.year(), d.month() as u32, d.day_of_month() as u32)
        {
            st_cal.borrow_mut().date = nd;
            up_c();
            up_s();
        }
    });

    let st_draw = state.clone();
    drawing_area_1.set_draw_func(move |_, cr, width, height| {
        let s = st_draw.borrow();
        if let Ok(pb) = gdk_pixbuf::Pixbuf::from_file(&s.solar_file) {
            cr.save().unwrap();
            cr.translate(s.x_offset as f64, s.y_offset as f64);
            cr.scale(s.x_scale, s.y_scale);
            cr.set_source_pixbuf(&pb, 0.0, 0.0);
            cr.paint().unwrap();
            cr.restore().unwrap();
        }

        cr.set_source_rgb(0.0, 0.7, 1.0);
        let dt_base = Utc
            .with_ymd_and_hms(s.date.year(), s.date.month(), s.date.day(), 12, 0, 0)
            .unwrap();
        if let Ok(spa::SunriseAndSet::Daylight(sr, ss)) =
            sunrise_and_set::<StdFloatOps>(dt_base, s.lat, s.lon)
        {
            let start_m = sr.hour() as f64 * 60.0 + sr.minute() as f64;
            let end_m = ss.hour() as f64 * 60.0 + ss.minute() as f64;
            for i in 0..(width as i32) {
                let m_total = start_m + (i as f64 / width as f64) * (end_m - start_m);
                let h = (m_total / 60.0) as u32 % 24;
                let m = (m_total % 60.0) as u32;
                if let Some(dt) = Utc
                    .with_ymd_and_hms(s.date.year(), s.date.month(), s.date.day(), h, m, 0)
                    .single()
                {
                    if let Ok(pos) = solar_position::<StdFloatOps>(dt, s.lat, s.lon) {
                        let y = (pos.zenith_angle / 30.0) * height as f64 - 5100.0;
                        println!("{y}");
                        if i == 0 {
                            cr.move_to(i as f64 - 55.0, y);
                        } else {
                            cr.line_to(i as f64 - 55.0, y);
                        }
                    }
                }
            }
            let _ = cr.stroke();
        }
    });

    update_slider();
    update_canvas();
    window.show();
}

fn create_file_picker_row(label: &str, path: &str) -> (Box, Entry, Image) {
    let row = Box::new(Orientation::Horizontal, 10);
    let entry = Entry::builder().text(path).hexpand(true).build();
    let btn = gtk4::Button::with_label("Обзор");
    let img = Image::builder().pixel_size(60).build();
    if let Ok(pb) = gdk_pixbuf::Pixbuf::from_file_at_scale(path, 60, 60, true) {
        img.set_from_pixbuf(Some(&pb));
    }
    row.append(&Label::new(Some(label)));
    row.append(&entry);
    row.append(&btn);
    row.append(&img);
    (row, entry, img)
}
