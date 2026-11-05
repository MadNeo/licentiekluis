#![cfg_attr(windows, windows_subsystem = "windows")]
#[cfg(windows)] mod ui;
#[cfg(windows)] fn main(){if let Err(e)=ui::run(){native_windows_gui::simple_message("LicentieKluis",&format!("Starten mislukt: {e:#}"));}}
#[cfg(not(windows))] fn main(){eprintln!("De native client is voor Windows. De server draait afzonderlijk op Linux.");}
