use ratatui::widgets::Block; 
use ratatui::widgets::Borders; 
use rpu::core::       Core; 
use ratatui::layout:: Constraint; 
use ratatui::         DefaultTerminal; 
use rpu::devices::    Device;
use ratatui::layout:: Direction; 
use crossterm::event::Event; 
use crossterm::event::KeyCode; 
use ratatui::         Frame; 
use ratatui::layout:: Layout; 
use ratatui::text::   Line;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph; 
use clap::            Parser;
use std::path::       PathBuf;
use rpu::programs::   Program; 
use rpu::core::       RAM;
use ratatui::layout:: Rect; 
use color_eyre::      Result; 
use ratatui::widgets::Row; 
use ratatui::text::   Span;
use ratatui::style::  Style; 
use ratatui::style::  Stylize; 
use ratatui::widgets::Table; 
use ratatui::widgets::TableState; 
use rpu::             devices;
use crossterm::       event; 
use std::             fs; 

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    source: PathBuf
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    let source = fs::read_to_string(&args.source)?;
    let mut core = Core::new();
    let program = Program::try_compile(&source).unwrap();
    core.load_program(&program).unwrap();


    color_eyre::install()?;
    let terminal = ratatui::init();
    let computer = Computer::new(core, program);
    let result = run(terminal, computer);
    ratatui::restore();
    result
}

fn run(
    mut terminal: DefaultTerminal,
    mut computer: Computer,
) -> Result<()> {
    loop {
        terminal.draw(|f| { render(&mut computer,f); })?;
        match event::read()? {
            Event::Key(ke) => {  
                match ke.code {
                    KeyCode::Esc => {
                        break Ok(())
                    },
                    KeyCode::Char('q') => {
                        break Ok(())
                    },
                    KeyCode::Down => {
                        let new = computer.code_list_state
                            .offset() + 1;
                        let current = computer.code_list_state
                            .offset_mut();
                        *current = new;
                    },
                    KeyCode::Up => {
                        let old = computer.code_list_state
                            .offset();
                        let new = if old == 0 {
                            0
                        } else {
                            old - 1
                        };
                        let current = computer.code_list_state
                            .offset_mut();
                        *current = new;
                    },
                    KeyCode::PageDown => {
                        computer.memory_table_state
                            .select_next();
                    },
                    KeyCode::PageUp => {
                        computer.memory_table_state
                            .select_previous();
                    },
                    KeyCode::Char('n') => {
                        let mut devices: Vec<&mut dyn Device> = vec![
                            &mut computer.lcd0,
                            &mut computer.lcd1,
                        ];
                        let r = computer.core
                            .execute_single_instruction(
                                &mut devices
                            );
                        match r {
                            Ok(false) => { continue; },
                            Ok(true) => { break Ok(()) },
                            Err(e) => {
                                computer.core.tty +=
                                    &format!("{:?}\n", e);
                                continue;
                            }
                        }
                    },
                    _ => {},
                }
            },
            _ => {}
        }
    }
}

struct Computer {
    core: Core,
    program: Program,
    lcd0: LCD,
    lcd1: LCD,
    code_list_state: ListState,
    memory_table_state: TableState,
}

impl Computer {
    fn new(core: Core, program: Program) -> Self {
        Self {
            core,
            program,
            lcd0: LCD::default(),
            lcd1: LCD::default(),
            code_list_state: ListState::default(),
            memory_table_state: TableState::new()
                .with_selected(Some(0)),
        }
    }
}

struct Layouts {
    code: Rect,
    help: Rect,
    lcd0: Rect,
    lcd1: Rect,
    memory: Rect,
    printer: Rect,
    power_led: Rect,
    registers: Rect,
    special_registers: Rect,
}

impl Layouts {
    fn new(frame: &Frame) -> Self {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(100),
                Constraint::Min(31),
                Constraint::Min(55),
            ])
            .split(frame.area());

        let lefthand_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(4)
            ])
            .split(layout[0]);
        let code = lefthand_layout[0];
        let help = lefthand_layout[1];

        let devices_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(7),
                Constraint::Length(7),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .split(layout[1]);
        let lcd0 = devices_layout[0];
        let lcd1 = devices_layout[1];
        let power_led = devices_layout[2];
        let printer = devices_layout[3];

        let tools_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Fill(1),
            ])
            .split(layout[2]);
        let registers = tools_layout[0];
        let special_registers = tools_layout[1];
        let memory = tools_layout[2];

        Self {
            code,
            help,
            lcd0,
            lcd1,
            memory,
            registers, 
            special_registers,
            printer,
            power_led,
        }
    }
}

fn render(computer: &mut Computer, frame: &mut Frame) {
    let layouts = Layouts::new(frame);


    render_code(
        &computer.program,
        computer.core.register_file.pc,
        &mut computer.code_list_state,
        layouts.code,
        frame,
        "Code"
    );
    render_help(layouts.help, frame, "Help");
    computer.lcd0.render(
        layouts.lcd0,
        frame,
        "LCD0 (dvc 0)"
    );
    computer.lcd1.render(
        layouts.lcd1,
        frame,
        "LCD1 (dvc 1)"
    );
    render_led(
        computer.core.power,
        layouts.power_led,
        frame,
        "Power"
    );
    render_printer(
        &computer.core.tty,
        layouts.printer,
        frame,
        "Error Console"
    );

    let gp_registers = vec![
        ("gp0", computer.core.register_file.gp0),
        ("gp1", computer.core.register_file.gp1),
        ("gp2", computer.core.register_file.gp2),
        ("gp3", computer.core.register_file.gp3),
        ("gp4", computer.core.register_file.gp4),
        ("gp5", computer.core.register_file.gp5),
        ("gp6", computer.core.register_file.gp6),
        ("gp7", computer.core.register_file.gp7),
    ];
    render_registers(
        gp_registers,
        layouts.registers, 
        frame,
        "General Purpose Registers"
    );


    let sp_registers = vec![
        ("ans", computer.core.register_file.ans),
        ("dvc", computer.core.register_file.dvc),
        ("pc", computer.core.register_file.pc),
        ("sp", computer.core.register_file.sp),
    ];
    render_registers(
        sp_registers,
        layouts.special_registers,
        frame,
        "Special Purpose Registers"
    );


    // Memory
    render_memory(
        &computer.core.memory,
        &mut computer.memory_table_state,
        layouts.memory,
        frame,
        "Memory"
    );
}

fn render_code(
    program: &Program,
    pc: u16,
    state: &mut ListState,
    area: Rect, 
    frame: &mut Frame,
    title: &str,
) {
    let current_line = program.source_addrs.get(&pc);
    state.select(current_line.copied());
    let items = program.source_lines.clone();
    let list = List::new(items)
        .block(common_block(title))
        .highlight_style(Style::new().italic().red());

    frame.render_stateful_widget(list, area, state);
}

fn render_help(
    area: Rect,
    frame: &mut Frame,
    title: &str
) {
    let text_n = vec![
        Line::from(vec![
            Span::styled("n", Style::new().bold()),
            Span::raw(" - execute next instruction")
        ])
    ];
    let text_q = vec![
        Line::from(vec![
            Span::styled("q", Style::new().bold()),
            Span::raw(" - exit this program")
        ]),
    ];
    let text_up = vec![
        Line::from(vec![
            Span::styled("Up/Down", Style::new().bold()),
            Span::raw(" - scroll code window")
        ])
    ];
    let text_pgup = vec![
        Line::from(vec![
            Span::styled("PgUp/PgDown", Style::new().bold()),
            Span::raw(" - scroll mem window")
        ])
    ];
    let rows = [
        Row::new([text_n, text_q]),
        Row::new([text_up, text_pgup])
    ];
    let widths = vec![
        Constraint::Length(28), Constraint::Length(31)
    ];
    let table = Table::new(rows, widths)
        .column_spacing(4)
        .block(common_block(title));
    frame.render_widget(table, area);
}

fn render_led(
    power: bool,
    area: Rect,
    frame: &mut Frame,
    title: &str,
) {
    let content = match power {
        true =>  " ON  ",
        false => " OFF "
    };
    let style = match power {
        true => Style::new().black().on_green(),
        false => Style::new().white().on_red(),
    };
    let text = vec![
        Line::from(vec![
            Span::styled(
                String::from(content),
                style
            )
        ])
    ];
    let paragraph = Paragraph::new(text)
        .block(common_block(title))
        .centered();
    frame.render_widget(paragraph, area);
}

#[derive(Default)]
struct LCD {
    value: u16
}

impl devices::Device for LCD {
    fn write(&mut self, value: u16)
        -> Result<(), devices::Error>
    {
        self.value = value;
        Ok(())
    }

    fn read(&mut self)
        -> Result<Option<u16>, devices::Error>
    {
        Err(devices::Error::Read(
            String::from("The LCD isn't an input")
        ))
    }
}

impl LCD {
    fn render(
        &self,
        area: Rect,
        frame: &mut Frame,
        title: &str
    ) {
        let font_definition = include_str!("../lcd_font.txt");
        let mut lcd_font: Vec<Vec<&str>> = vec![];
        let mut current_lcd_char: Vec<&str> = vec![];
        for (n, text) in font_definition.lines().enumerate() {
            current_lcd_char.push(text);
            if ((n + 1) % 5) == 0 {
                lcd_font.push(current_lcd_char);
                current_lcd_char = vec![];
            }
        }

        let value = format!("{:0>5}", self.value);
        let mut content = String::new();
        for row in 0..5 {
            for c in value.chars() {
                let char_id = match c {
                    '0' => 0,
                    '1' => 1,
                    '2' => 2,
                    '3' => 3,
                    '4' => 4,
                    '5' => 5,
                    '6' => 6,
                    '7' => 7,
                    '8' => 8,
                    '9' => 9,
                    _ => panic!()
                };

                content.push_str(lcd_font[char_id][row]);
            }
            content.push_str("\n");
        }

        let paragraph = Paragraph::new(content)
            .block(common_block(title));
        frame.render_widget(paragraph, area);
    }
}


fn render_printer(
    text: &str,
    area: Rect,
    frame: &mut Frame,
    title: &str
) {
    let paragraph = Paragraph::new(text.to_string())
        .block(common_block(title));
    frame.render_widget(paragraph, area);
}

fn render_registers(
    pairs: Vec<(&str, u16)>,
    area: Rect,
    frame: &mut Frame,
    title: &str
) {
    let cells: Vec<String> = (&pairs).into_iter().map(|(_, val)| {
        format!("{:5}", val)
    }).collect();
    let rows = [Row::new(cells)];
    let widths: Vec<Constraint> = (&pairs).into_iter().map(|_| {
        Constraint::Length(5)
    }).collect();
    let block = common_block(title);
    let header_cells: Vec<String> = (&pairs).into_iter().map(|(name, _)| {
        format!("{:>5}", name)
    }).collect();
    let header = Row::new(header_cells)
        .style(Style::new().bold());
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .header(header)
        .block(block);
    frame.render_widget(table, area);
}

fn render_memory(
    memory: &[u8; RAM],
    state: &mut TableState,
    area: Rect,
    frame: &mut Frame,
    title: &str,
) {
    let mut rows: Vec<Row> = vec![];
    let mut current_row: Vec<String> = vec![];
    current_row.push(String::from("    0"));
    for (addr, byte) in memory.iter().enumerate() {
        current_row.push(format!("{:5}", byte));
        if (addr + 1) % 8 == 0 {
            let style = match (addr / 8) % 2 == 0 {
                true => Style::default(),
                false => Style::default(),
            };
            rows.push(
                Row::new(current_row).style(style)
            );
            current_row = vec![];
            current_row.push(format!("{:>5}", addr+1));
        }
    }
    let widths = [
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
    ];
    let table = Table::new(rows, widths)
        .column_spacing(1)
        .header(
            Row::new(vec![
                "ADDR ",
                "   +0",
                "   +1",
                "   +2",
                "   +3",
                "   +4",
                "   +5",
                "   +6",
                "   +7",
            ])
                .style(Style::new().bold())
                .bottom_margin(1)
        )
        .row_highlight_style(Style::new().red().italic())
        .block( common_block(title));
    frame.render_stateful_widget(table, area, state);

}

fn common_block(title: &str) -> Block {
    let title = format!("[{title}]");
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().blue())
}
