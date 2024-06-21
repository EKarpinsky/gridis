use iced::keyboard::{Key, Location};
use iced::widget::{column, Button, Column, Row, Text};
use iced::Renderer;
use iced::{
    executor, keyboard, Application, Command, Element, Length, Settings, Subscription, Theme,
};
use std::f64;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};

const WS_EX_TOOLWINDOW_U32: u32 = WS_EX_TOOLWINDOW.0;
const WS_EX_APPWINDOW_U32: u32 = WS_EX_APPWINDOW.0;
//windowedge, noredirectionbitmap, contexthelp
const WS_EX_WINDOWEDGE_U32: u32 = WS_EX_WINDOWEDGE.0;
const WS_EX_NOREDIRECTIONBITMAP_U32: u32 = WS_EX_NOREDIRECTIONBITMAP.0;
const WS_EX_CONTEXTHELP_U32: u32 = WS_EX_CONTEXTHELP.0;

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
    Undo,
    // EventOccurred(iced::Event),
}

struct Style;

impl Application for LayoutApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (LayoutApp, Command<Message>) {
        let mut windows: Vec<HWND> = Vec::new();
        unsafe {
            EnumWindows(Some(enum_windows_proc), LPARAM(&mut windows as *const Vec<HWND> as isize))
                .expect("EnumWindows failed");

            // arrange_windows(windows);
        }

        (
            LayoutApp {
                windows,
                initial_positions: vec![],
                show_gui: true,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Window Layout Tool")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ToggleGui => {
                self.show_gui = !self.show_gui;
            }
            Message::LayoutSelected(index) => {
                println!("Selected layout {}", index);
                // Add your logic to apply the selected layout
            }
            Message::ArrangeWindows => unsafe {
                arrange_windows(&self.windows, &mut self.initial_positions);
            },
            Message::Undo => unsafe {
                undo_window_positions(&self.initial_positions);
            },
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // 4 possibilities of arrangements based on the windows provided, shown via graphical blocks
        // for example, if 4 windows, show 1 large 3 small, 4 equal, etc
        // first get the number of windows:
        let num_of_windows = self.windows.len();
        // then create the possibilities, with row and cols and widths and heights of windows
        type Layout = (i32, i32, i32, i32); // (row, col, width, height) of the window

        // then create the layouts based on the number of windows
        // there should be a button for each layout possibility
        // the possibilities depend on the amount of windows

        // math equation for possible layouts based on the possible width/height percentages and the number of windows:
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
                    let col_width = (5120.0 * width_percentage) as i32;
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
        let layouts = get_layouts(
            num_of_windows as i32,
            vec![0.25, 0.33, 0.5, 0.66, 0.75, 1.0],
            vec![0.5, 1.0],
        );

        let mock_num_of_windows = 6;
        let mock_num_of_rows = 2;
        let mock_num_of_rows_iter = (0..mock_num_of_rows).collect::<Vec<i32>>();
        let mock_num_of_windows_per_row: i32 = mock_num_of_windows / mock_num_of_rows;
        let mock_num_of_windows_per_row_iter =
            (0..mock_num_of_windows_per_row).collect::<Vec<i32>>();
        let mock_window_width_percentage = 0.25;
        let mock_window_height_percentage = 1 / mock_num_of_rows;
        let mock_window_row = 0;
        let mock_window_col = 0;

        let row_elements = mock_num_of_rows_iter.iter().map(|row| {
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
        let desktop: Column<Message> = Column::<Message>::with_children(row_elements).into();

        let main_col = Column::new()
            .push(Button::new("Toggle GUI").on_press(Message::ToggleGui))
            .push(Button::new("Arrange Windows").on_press(Message::ArrangeWindows))
            .push(Button::new("Undo").on_press(Message::Undo))
            .push(desktop)
            .into();

        main_col
    }
}

unsafe fn undo_window_positions(initial_positions: &Vec<(HWND, RECT)>) {
    for (hwnd, rect) in initial_positions.iter() {
        SetWindowPos(
            *hwnd,
            HWND_TOP,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .expect("Failed to set window pos");
    }
}

unsafe fn arrange_windows(windows: &Vec<HWND>, initial_positions: &mut Vec<(HWND, RECT)>) {
    {
        // update initial_positions to store the current positions of the windows
        initial_positions.clear();
        for hwnd in windows.iter() {
            let mut rect = RECT::default();
            GetWindowRect(*hwnd, &mut rect).expect("TODO: panic message");
            initial_positions.push((*hwnd, rect));
        }
        // set undo button to enabled via onpress
    }
    let mut x = 0;
    let mut y = 0;
    let num_of_rows = 2;
    let amount_of_windows_in_first_row =
        f64::floor(windows.len() as f64 / num_of_rows as f64) as i32;
    let amount_of_windows_in_second_row = windows.len() as i32 - amount_of_windows_in_first_row;
    let window_width_first_row = 5120 / amount_of_windows_in_first_row;
    let window_width_second_row = 5120 / amount_of_windows_in_second_row;
    for hwnd in windows.iter() {
        let mut rect = RECT::default();
        let _ = ShowWindow(*hwnd, SW_SHOW);
        GetWindowRect(*hwnd, &mut rect).expect("TODO: panic message");
        SetWindowPos(
            *hwnd,
            HWND_TOP,
            x,
            y,
            if y == 0 {
                window_width_first_row
            } else {
                window_width_second_row
            },
            1440 / num_of_rows,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .expect("TODO: panic message");
        x += if y == 0 {
            window_width_first_row
        } else {
            window_width_second_row
        };
        if x >= 5121
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
    }
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, l_param: LPARAM) -> BOOL {
    let mut windows = l_param.0 as *mut Vec<HWND>;
    let windows = unsafe { &mut *windows };
    let mut title: [u16; 512] = [0; 512];
    let length = GetWindowTextW(hwnd, &mut title);
    // display information about the window:
    let mut info = WINDOWINFO::default();
    GetWindowInfo(hwnd, &mut info).expect("TODO: panic message");
    let is_visible = IsWindowVisible(hwnd).as_bool();
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

    if is_visible && length > 0 && (ex_style & WS_EX_TOOLWINDOW_U32 == 0)
    // && (ex_style & WS_EX_APPWINDOW_U32 != 0)
    {
        let title = String::from_utf16_lossy(&title[..length as usize]);
        let mut wp = WINDOWPLACEMENT::default();
        let mut rect = RECT::default();
        GetWindowPlacement(hwnd, &mut wp).expect("TODO: panic message");
        GetWindowRect(hwnd, &mut rect).expect("TODO: panic message");

        if !title.is_empty()
            && wp.showCmd > 0
            && rect.left + rect.top + rect.right + rect.bottom != 0
            //     and ex_style is either 256, 262416, or has the title "WhatsApp"
            && (WS_EX_WINDOWEDGE_U32 == ex_style
            || (ex_style & WS_EX_APPWINDOW_U32 != 0)
            || title == "WhatsApp")
        {
            println!("Adding window to list: {}", title);

            windows.push(hwnd);
        }
    }
    true.into() // Continue enumeration
}

fn main() -> iced::Result {
    LayoutApp::run(Settings::default())
}
