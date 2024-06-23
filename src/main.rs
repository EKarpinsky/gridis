use display_info::DisplayInfo;
use iced::widget::{Button, Column, Row, Text};
use iced::{executor, window, Application, Command, Element, Pixels, Settings, Size, Theme};
use std::f64;
use windows::core::Result;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};
use windows_core::HRESULT;

const WS_EX_TOOLWINDOW_U32: u32 = WS_EX_TOOLWINDOW.0;
const WS_EX_APPWINDOW_U32: u32 = WS_EX_APPWINDOW.0;
const WS_EX_WINDOWEDGE_U32: u32 = WS_EX_WINDOWEDGE.0;

#[derive(Default)]
struct LayoutApp {
    windows: Vec<HWND>,
    initial_positions: Vec<(HWND, RECT)>,
    show_gui: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    ToggleGui,
    LayoutSelected(usize),
    ArrangeWindows,
    SwapMonitors,
    Undo,
    // EventOccurred(iced::Event),
}

impl Application for LayoutApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let windows = Self::enumerate_windows();

        (
            Self {
                windows,
                initial_positions: vec![],
                show_gui: true,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Gridis")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ToggleGui => {
                self.show_gui = !self.show_gui;
            }
            Message::LayoutSelected(index) => {
                println!("Selected layout {}", index);
                // apply the selected layout
            }
            Message::ArrangeWindows => {
                Self::arrange_windows(&self.windows, &mut self.initial_positions);
            }
            Message::SwapMonitors => {
                Self::swap_monitors(&self.windows, &mut self.initial_positions);
            }
            Message::Undo => {
                Self::undo_window_positions(&self.initial_positions);
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        type Layout = (i32, i32, i32, i32); // (row, col, width, height)

        fn get_layouts(
            num_of_windows: i32,
            possible_width_percentages: Vec<f64>,
            possible_height_percentages: Vec<f64>,
        ) -> Vec<Layout> {
            let mut layouts = vec![];
            for width_percentage in possible_width_percentages.iter() {
                for height_percentage in possible_height_percentages.iter() {
                    let num_of_cols_per_row = if *height_percentage == 1.0 {
                        num_of_windows
                    } else {
                        num_of_windows / 2
                    };
                    let num_of_rows = (1.0 / height_percentage) as i32;
                    let col_width = (2560.0 * width_percentage) as i32;
                    let row_height = (1440.0 * height_percentage) as i32;

                    for row in 0..num_of_rows {
                        for col in 0..num_of_cols_per_row {
                            layouts.push((row, col, col_width, row_height));
                        }
                    }
                }
            }
            layouts
        }

        // now we create square button areas showcasing each of the layouts, with the widths according
        // let layouts = get_layouts(
        //     num_of_windows as i32,
        //     vec![0.25, 0.33, 0.5, 0.66, 0.75, 1.0],
        //     vec![0.5, 1.0],
        // );

        let mock_num_of_windows = 6;
        let mock_num_of_rows = 2;
        let mock_num_of_rows_iter = (0..mock_num_of_rows).collect::<Vec<i32>>();
        let mock_num_of_windows_per_row: i32 = mock_num_of_windows / mock_num_of_rows;
        let mock_num_of_windows_per_row_iter =
            (0..mock_num_of_windows_per_row).collect::<Vec<i32>>();
        // let mock_window_width_percentage = 0.25;
        // let mock_window_height_percentage = 1 / mock_num_of_rows;
        // let mock_window_row = 0;
        // let mock_window_col = 0;

        let row_elements = mock_num_of_rows_iter.iter().map(|_row| {
            let row_window_elements = mock_num_of_windows_per_row_iter
                .iter()
                .map(|window_in_row| {
                    Button::<Message>::new(Text::new(format!("Window {}", window_in_row)))
                        .on_press(Message::LayoutSelected(0))
                        .into()
                })
                .collect::<Vec<Element<Message>>>();

            Row::from_vec(row_window_elements).into()
        });
        let desktop: Column<Message> = Column::<Message>::with_children(row_elements);

        let main_col = Column::new()
            .push(Button::new("Toggle GUI").on_press(Message::ToggleGui))
            .push(Button::new("Arrange Windows").on_press(Message::ArrangeWindows))
            .push(Button::new("Swap Monitors").on_press(Message::SwapMonitors))
            .push(Button::new("Undo").on_press(Message::Undo))
            .push(desktop)
            .into();

        main_col
    }
}

impl LayoutApp {
    fn enumerate_windows() -> Vec<HWND> {
        let mut windows: Vec<HWND> = Vec::new();
        let ptr = &mut windows as *mut Vec<HWND> as isize;
        unsafe {
            if !EnumWindows(Some(Self::enum_windows_proc), LPARAM(ptr)).is_ok() {
                eprintln!("Enumerating windows failed");
            }
        }
        windows
    }

    fn undo_window_positions(initial_positions: &Vec<(HWND, RECT)>) {
        for (hwnd, rect) in initial_positions.iter() {
            if !Self::safe_undo_set_window_pos(hwnd, rect).is_ok() {
                eprintln!("SetWindowPos failed for window {:?}", hwnd);
                continue;
            }
        }
    }

    fn safe_undo_set_window_pos(hwnd: &HWND, rect: &RECT) -> Result<()> {
        unsafe {
            SetWindowPos(
                *hwnd,
                HWND_TOP,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        }
    }

    fn swap_monitors(windows: &Vec<HWND>, initial_positions: &mut Vec<(HWND, RECT)>) {
        // get recs of all windows and assign them to either top or bottom monitor. top monitor is if the "top" of the window's rect is less than 0
        const TOP_MONITOR_BOUNDARY: i32 = 0 - 15;
        let monitors_info = DisplayInfo::all().unwrap();
        struct MonitorInfo {
            x: i32,
            y: i32,
            width: u32,
            height: u32,
            is_primary: bool,
        }
        impl MonitorInfo {
            fn new(x: i32, y: i32, width: u32, height: u32, is_primary: bool) -> Self {
                Self {
                    x,
                    y,
                    width,
                    height,
                    is_primary,
                }
            }
        }

        let mut monitors: Vec<MonitorInfo> = vec![];

        for monitor_info in monitors_info.iter() {
            monitors.push(MonitorInfo::new(
                monitor_info.x,
                monitor_info.y,
                monitor_info.width,
                monitor_info.height,
                monitor_info.is_primary,
            ));
        }

        initial_positions.clear();

        for hwnd in windows.iter() {
            let rect = match Self::safe_get_window_rect(hwnd) {
                Ok(rect) => rect,
                Err(_) => {
                    eprintln!("GetWindowRect failed for {:?}", hwnd);
                    continue;
                }
            };

            let monitor = monitors
                .iter()
                .find(|m| m.is_primary != (rect.top <= TOP_MONITOR_BOUNDARY))
                .unwrap();

            // get title
            let mut title_char_array: [u16; 512] = [0; 512];
            let length = Self::safe_get_window_title_length(&hwnd, &mut title_char_array);
            let title = String::from_utf16_lossy(&title_char_array[..length as usize]);

            // x and y are proportional. if rect.left was equal to monitor's x it means it was on the monitor's left edge, so we want to m ove it to the other monitor's left edge as well
            // etc for y, right edge, other cases, etc
            let left_relative_to_monitor = (rect.left - monitor.x) / monitor.width as i32;
            let top_relative_to_monitor = (rect.top - monitor.y) / monitor.height as i32;
            let width_relative_to_monitor = (rect.right - rect.left) as u32 / monitor.width;
            let height_relative_to_monitor = (rect.bottom - rect.top) as u32 / monitor.height;
            let new_x = if monitor.is_primary {
                monitors.iter().find(|m| !m.is_primary).unwrap().x + left_relative_to_monitor
            } else {
                monitors.iter().find(|m| m.is_primary).unwrap().x + left_relative_to_monitor
            };
            let new_y = if monitor.is_primary {
                monitors.iter().find(|m| !m.is_primary).unwrap().y + top_relative_to_monitor
            } else {
                monitors.iter().find(|m| m.is_primary).unwrap().y + top_relative_to_monitor
            };
            let new_width = if monitor.is_primary {
                monitors.iter().find(|m| !m.is_primary).unwrap().width * width_relative_to_monitor
            } else {
                monitors.iter().find(|m| m.is_primary).unwrap().width * width_relative_to_monitor
            } as i32;
            let new_height = if monitor.is_primary {
                monitors.iter().find(|m| !m.is_primary).unwrap().height * height_relative_to_monitor
            } else {
                monitors.iter().find(|m| m.is_primary).unwrap().height * height_relative_to_monitor
            } as i32;

            if Self::safe_set_window_pos(
                new_x,
                new_y,
                new_width,
                new_height,
                SWP_NOZORDER | SWP_NOACTIVATE,
                hwnd,
            )
            .is_err()
            {
                eprintln!("SetWindowPos failed for {:?}", hwnd);
                continue;
            }

            initial_positions.push((*hwnd, rect));
        }
    }

    fn arrange_windows(windows: &Vec<HWND>, initial_positions: &mut Vec<(HWND, RECT)>) {
        let mut x = 0;
        let mut y = 0;
        let num_of_rows = 2;
        let amount_of_windows_in_first_row =
            (windows.len() as f64 / num_of_rows as f64).floor() as i32;
        if amount_of_windows_in_first_row == 0 {
            eprintln!("Not enough windows to arrange");
            return;
        }

        let amount_of_windows_in_second_row = windows.len() as i32 - amount_of_windows_in_first_row;
        let window_width_first_row = 2560 / amount_of_windows_in_first_row;
        let window_width_second_row = 2560 / amount_of_windows_in_second_row;
        initial_positions.clear();
        for hwnd in windows.iter() {
            if !Self::safe_show_window(hwnd).as_bool() {
                eprintln!("ShowWindow failed for {:?}", hwnd);
                continue;
            }
            let window_rect = match Self::safe_get_window_rect(hwnd) {
                Ok(rect) => rect,
                Err(_) => {
                    eprintln!("GetWindowRect failed for {:?}", hwnd);
                    continue;
                }
            };

            if Self::set_window_row_pos(
                x,
                y,
                num_of_rows,
                window_width_first_row,
                window_width_second_row,
                hwnd,
            )
            .is_err()
            {
                eprintln!("SetWindowPos failed for {:?}", hwnd);
                continue;
            }

            x += if y == 0 {
                window_width_first_row
            } else {
                window_width_second_row
            };

            if x >= 2561
                - if y == 0 {
                    window_width_first_row
                } else {
                    window_width_second_row
                }
            {
                x = 0;
                y += 1440 / num_of_rows;
            }
            println!("X: {}", x);
            println!("Y: {}", y);
            initial_positions.push((*hwnd, window_rect));
        }
    }

    fn set_window_row_pos(
        x: i32,
        y: i32,
        num_of_rows: i32,
        window_width_first_row: i32,
        window_width_second_row: i32,
        hwnd: &HWND,
    ) -> Result<()> {
        Self::safe_set_window_pos(
            x,
            y,
            if y == 0 {
                window_width_first_row
            } else {
                window_width_second_row
            },
            1440 / num_of_rows,
            SWP_NOZORDER | SWP_NOACTIVATE,
            hwnd,
        )
    }

    fn safe_set_window_pos(
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        uflags: SET_WINDOW_POS_FLAGS,
        hwnd: &HWND,
    ) -> Result<()> {
        unsafe { SetWindowPos(*hwnd, HWND_TOP, x, y, cx, cy, uflags) }
    }

    fn safe_show_window(hwnd: &HWND) -> BOOL {
        unsafe { ShowWindow(*hwnd, SW_SHOW) }
    }

    fn safe_get_window_rect(hwnd: &HWND) -> Result<RECT> {
        let mut rect = RECT::default();

        match unsafe { GetWindowRect(*hwnd, &mut rect) } {
            Ok(_) => Ok(rect),
            Err(err) => Err(err),
        }
    }

    extern "system" fn enum_windows_proc(hwnd: HWND, l_param: LPARAM) -> BOOL {
        let windows = match Self::extract_windows_from_lparam(l_param) {
            Ok(windows) => windows,
            Err(_) => return false.into(),
        };
        let mut title_char_array: [u16; 512] = [0; 512];
        let length = Self::safe_get_window_title_length(&hwnd, &mut title_char_array);
        let is_visible = Self::safe_is_window_visible(&hwnd);
        let ex_style = Self::safe_get_window_long_w(&hwnd);

        if is_visible && length > 0 && (ex_style & WS_EX_TOOLWINDOW_U32 == 0) {
            let title = String::from_utf16_lossy(&title_char_array[..length as usize]);
            let mut wp = WINDOWPLACEMENT::default();
            RECT::default();

            unsafe {
                GetWindowPlacement(hwnd, &mut wp).expect("TODO: panic message");
            }

            let rect = match Self::safe_get_window_rect(&hwnd) {
                Ok(rect) => rect,
                Err(_) => {
                    eprintln!("GetWindowRect failed for {:?}", hwnd);
                    return false.into();
                }
            };

            if !title.is_empty()
                && wp.showCmd > 0
                && rect.left + rect.top + rect.right + rect.bottom != 0
                && (WS_EX_WINDOWEDGE_U32 == ex_style
                    || (ex_style & WS_EX_APPWINDOW_U32 != 0)
                    || title == "WhatsApp")
            {
                println!("Adding window to list: {}", title);
                if title.contains("WhatsApp") {
                    println!("Rect: {:?}", rect);
                }

                windows.push(hwnd);
            }
        }
        true.into() // Continue enumeration
    }

    fn safe_get_window_long_w(hwnd: &HWND) -> u32 {
        unsafe { GetWindowLongW(*hwnd, GWL_EXSTYLE) as u32 }
    }

    fn safe_is_window_visible(hwnd: &HWND) -> bool {
        unsafe { IsWindowVisible(*hwnd).as_bool() }
    }

    fn safe_get_window_title_length(hwnd: &HWND, title: &mut [u16]) -> i32 {
        unsafe { GetWindowTextW(*hwnd, title) }
    }

    fn extract_windows_from_lparam<'a>(l_param: LPARAM) -> Result<&'a mut Vec<HWND>> {
        let windows_ptr = l_param.0 as *mut Vec<HWND>;
        if windows_ptr.is_null() {
            return Err(HRESULT::from_win32(ERROR_INVALID_PARAMETER.0).into());
        }
        let windows = unsafe { &mut *windows_ptr };
        Ok(windows)
    }
}

fn main() -> iced::Result {
    LayoutApp::run(Settings {
        id: None,
        window: window::Settings {
            size: Size {
                width: 700f32,
                height: 800f32,
            },
            position: Default::default(),
            min_size: None,
            max_size: None,
            visible: true,
            resizable: false,
            decorations: true,
            transparent: true,
            level: Default::default(),
            icon: None,
            platform_specific: Default::default(),
            exit_on_close_request: true,
        },
        flags: (),
        fonts: Default::default(),
        default_font: Default::default(),
        default_text_size: Pixels::from(16.0),
        antialiasing: false,
    })
}
