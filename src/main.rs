use gio::AppInfo;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, ComboBoxText,
    FileChooserAction, FileChooserDialog, Orientation, ProgressBar, ResponseType, ScrolledWindow,
    TextBuffer, TextView,
};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

fn main() {
    // Disable GIO proxy modules to avoid Snap-related errors
    env::set_var("GIO_USE_PROXY", "none");

    // Initialize GTK application
    let app = Application::builder()
        .application_id("com.example.CppcheckGui")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    // Main window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Cppcheck GUI")
        .default_width(800)
        .default_height(600)
        .build();

    // State: selected project path
    let project_path = Rc::new(RefCell::new(None::<String>));

    // Layout container
    let vbox = GtkBox::new(Orientation::Vertical, 8);

    // Language selector
    let lang_combo = ComboBoxText::new();
    lang_combo.append_text("en");
    lang_combo.append_text("ru");
    lang_combo.set_active(Some(0));
    vbox.append(&lang_combo);

    // Directory chooser button
    let select_btn = Button::with_label("Select Project Directory");
    vbox.append(&select_btn);

    // Severity filters
    let chk_error = CheckButton::with_label("Error");
    chk_error.set_active(true);
    let chk_warning = CheckButton::with_label("Warning");
    chk_warning.set_active(true);
    let chk_style = CheckButton::with_label("Style");
    chk_style.set_active(false);
    let chk_performance = CheckButton::with_label("Performance");
    chk_performance.set_active(false);
    let hbox_checks = GtkBox::new(Orientation::Horizontal, 4);
    hbox_checks.append(&chk_error);
    hbox_checks.append(&chk_warning);
    hbox_checks.append(&chk_style);
    hbox_checks.append(&chk_performance);
    vbox.append(&hbox_checks);

    // Control buttons
    let btn_run = Button::with_label("Run Cppcheck");
    let btn_html = Button::with_label("Generate HTML");
    let btn_pdf = Button::with_label("Generate PDF");
    btn_html.set_sensitive(false);
    btn_pdf.set_sensitive(false);
    // Check utilities availability
    let html_ok = Command::new("which")
        .arg("cppcheck-htmlreport")
        .output()
        .is_ok();
    let pdf_tool: Option<String> = if Command::new("which").arg("google-chrome").output().is_ok() {
        Some("google-chrome".into())
    } else if Command::new("which")
        .arg("chromium-browser")
        .output()
        .is_ok()
    {
        Some("chromium-browser".into())
    } else {
        None
    };
    if !html_ok {
        btn_html.set_sensitive(false);
    }
    if pdf_tool.is_none() {
        btn_pdf.set_sensitive(false);
    }
    let hbox_btns = GtkBox::new(Orientation::Horizontal, 4);
    hbox_btns.append(&btn_run);
    hbox_btns.append(&btn_html);
    hbox_btns.append(&btn_pdf);
    vbox.append(&hbox_btns);

    // Log area
    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    let text_view = TextView::new();
    text_view.set_editable(false);
    text_view.set_vexpand(true);
    let buffer = text_view.buffer();
    scrolled.set_child(Some(&text_view));
    vbox.append(&scrolled);

    // Dependency install button
    let required = ["cppcheck", "cppcheck-htmlreport", "google-chrome"];
    let missing: Vec<String> = required
        .iter()
        .filter(|&&u| Command::new("which").arg(u).output().is_err())
        .map(|&u| u.into())
        .collect();
    if !missing.is_empty() {
        let install_btn = Button::with_label("Install Dependencies");
        vbox.append(&install_btn);
        let deps = missing.clone();
        let dep_buf = buffer.clone();
        let btn_clone = install_btn.clone();
        install_btn.connect_clicked(move |_| {
            append_text(&dep_buf, "Installing missing utilities...\n");
            let mut cmd = Command::new("sudo");
            cmd.args(["apt-get", "install", "-y"]).args(&deps);
            if let Ok(out) = cmd.output() {
                append_text(&dep_buf, &String::from_utf8_lossy(&out.stdout));
                append_text(&dep_buf, &String::from_utf8_lossy(&out.stderr));
            }
            btn_clone.set_sensitive(false);
        });
    }

    // Progress bar
    let progress = ProgressBar::new();
    vbox.append(&progress);

    window.set_child(Some(&vbox));
    window.present();

    // Directory chooser logic
    {
        let proj_clone = project_path.clone();
        let btn_clone = select_btn.clone();
        let win_clone = window.clone();
        select_btn.connect_clicked(move |_| {
            let dialog = FileChooserDialog::builder()
                .title("Select Project Directory")
                .action(FileChooserAction::SelectFolder)
                .transient_for(&win_clone)
                .modal(true)
                .build();
            dialog.add_buttons(&[
                ("Cancel", ResponseType::Cancel),
                ("Select", ResponseType::Accept),
            ]);
            let proj_inner = proj_clone.clone();
            let btn_inner = btn_clone.clone();
            dialog.connect_response(move |d, r| {
                if r == ResponseType::Accept {
                    if let Some(path) = d.file().and_then(|f| f.path()) {
                        let s = path.to_string_lossy().to_string();
                        *proj_inner.borrow_mut() = Some(s.clone());
                        btn_inner.set_label(&s);
                    }
                }
                d.close();
            });
            dialog.show();
        });
    }

    // Run cppcheck logic
    {
        let buf_run = buffer.clone();
        let chk_w = chk_warning.clone();
        let chk_s = chk_style.clone();
        let chk_p = chk_performance.clone();
        let proj_run = project_path.clone();
        let html_btn_clone = btn_html.clone();
        let pdf_btn_clone = btn_pdf.clone();
        let progress_clone = progress.clone();
        btn_run.connect_clicked(move |_| {
            if let Some(ref path) = *proj_run.borrow() {
                append_text(&buf_run, &format!("Running cppcheck on {}\n", path));
                progress_clone.set_fraction(0.0);
                let mut cmd = Command::new("cppcheck");
                let mut levels = Vec::new();
                if chk_w.is_active() {
                    levels.push("warning");
                }
                if chk_s.is_active() {
                    levels.push("style");
                }
                if chk_p.is_active() {
                    levels.push("performance");
                }
                if !levels.is_empty() {
                    cmd.arg(&format!("--enable={}", levels.join(",")));
                }
                cmd.arg(path);
                if let Ok(out) = cmd.output() {
                    append_text(&buf_run, &String::from_utf8_lossy(&out.stdout));
                    append_text(&buf_run, &String::from_utf8_lossy(&out.stderr));
                }
                progress_clone.set_fraction(1.0);
                html_btn_clone.set_sensitive(true);
                pdf_btn_clone.set_sensitive(true);
            }
        });
    }

    // Generate HTML report logic
    {
        let buf_html = buffer.clone();
        let proj_run = project_path.clone();
        btn_html.connect_clicked(move |_| {
            if let Some(ref path) = *proj_run.borrow() {
                append_text(&buf_html, &format!("Generating HTML report for {}\n", path));
                let project_name = Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("project");
                let xml_file = format!("{}/cppcheck.xml", path);
                if let Ok(out) = Command::new("cppcheck")
                    .args(&["--xml", "--xml-version=2", path])
                    .output()
                {
                    if fs::write(&xml_file, &out.stderr).is_err() {
                        append_text(&buf_html, "Failed to write XML report\n");
                        return;
                    }
                } else {
                    append_text(&buf_html, "Error running cppcheck --xml\n");
                    return;
                }
                let report_dir = format!("{}/html_report", path);
                if Command::new("cppcheck-htmlreport")
                    .args(&[
                        "--file",
                        &xml_file,
                        "--report-dir",
                        &report_dir,
                        "--source-dir",
                        path,
                        "--title",
                        &format!("Cppcheck report - {}", project_name),
                    ])
                    .output()
                    .is_err()
                {
                    append_text(&buf_html, "Error generating HTML report\n");
                    return;
                }
                append_text(
                    &buf_html,
                    &format!("HTML report saved to {}/html_report\n", path),
                );
                let index_uri = format!("file://{}/index.html", report_dir);
                if let Err(e) =
                    AppInfo::launch_default_for_uri(&index_uri, None::<&gio::AppLaunchContext>)
                {
                    append_text(&buf_html, &format!("Failed to open HTML report: {}\n", e));
                }
            }
        });
    }

    // Generate PDF report logic
    {
        let buf_pdf = buffer.clone();
        let proj_run = project_path.clone();
        let pdf_tool_clone = pdf_tool.clone();
        btn_pdf.connect_clicked(move |_| {
            if let Some(ref path) = *proj_run.borrow() {
                append_text(&buf_pdf, &format!("Generating PDF report for {}\n", path));
                let report_dir = format!("{}/html_report", path);
                let index_uri = format!("file://{}/index.html", report_dir);
                let pdf_file = format!("{}/report.pdf", path);
                if let Some(ref tool) = pdf_tool_clone {
                    if let Ok(_) = Command::new(tool)
                        .args(&[
                            "--headless",
                            "--disable-gpu",
                            &format!("--print-to-pdf={}", pdf_file),
                            &index_uri,
                        ])
                        .output()
                    {
                        if Path::new(&pdf_file).exists() {
                            append_text(&buf_pdf, &format!("PDF report saved to {}\n", pdf_file));
                            let pdf_uri = format!("file://{}", pdf_file);
                            if let Err(e) = AppInfo::launch_default_for_uri(
                                &pdf_uri,
                                None::<&gio::AppLaunchContext>,
                            ) {
                                append_text(
                                    &buf_pdf,
                                    &format!("Failed to open PDF report: {}\n", e),
                                );
                            }
                        } else {
                            append_text(&buf_pdf, "PDF report was not generated\n");
                        }
                    } else {
                        append_text(&buf_pdf, "Error generating PDF report\n");
                    }
                } else {
                    append_text(&buf_pdf, "No PDF utility available\n");
                }
            }
        });
    }
}

// Helper to append text to the TextView buffer
fn append_text(buffer: &TextBuffer, text: &str) {
    let mut iter = buffer.end_iter();
    buffer.insert(&mut iter, text);
}
