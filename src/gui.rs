macro_rules! import {
    [$($i:ident < $m:path),* $(,)?] => {
        $(use $m::{$i};)*
    };
}

import![
    Block < ratatui::widgets,
    Borders < ratatui::widgets,
    Color < ratatui::style,
    Core < crate::core,
    Constraint < ratatui::layout,
    DefaultTerminal < ratatui,
    Direction < ratatui::layout,
    Event < crossterm::event,
    Frame < ratatui,
    Layout < ratatui::layout,
    Paragraph < ratatui::widgets,
    Rect < ratatui::layout,
    Result < color_eyre,
    Row < ratatui::widgets,
    Style < ratatui::style,
    Stylize < ratatui::style,
    Table < ratatui::widgets,
    Write < std::io,
    event < crossterm,
    fs < std,
];

fn run<'tty,W: Write>(
    mut terminal: DefaultTerminal,
    mut app: App<'tty, W>,
) -> Result<()> {
    loop {
        terminal.draw(|f| { render(&app,f); })?;
        if matches!(event::read()?, Event::Key(_)) {
            match app.core.execute_single_instruction() {
                Ok(false) => { continue; },
                Ok(true) => { break Ok(()) },
                Err(e) => { eprintln!("{e}"); break Ok(()); }
            }
        }
    }
}

pub fn main(path: &str) -> Result<()> {
    let source = fs::read_to_string(path)?;
    let mut printer: Vec<u8> = vec![];
    let mut core = Core::new(&mut printer);
    core.load_source(&source).unwrap();


    color_eyre::install()?;
    let terminal = ratatui::init();
    let app = App::new(core);
    let result = run(terminal, app);
    ratatui::restore();
    result
}

struct App<'tty, W: Write> {
    core: Core<'tty,W>,
    code: String,
    memory: [u16; 65_536],
    lcd0: u16,
    lcd1: u16,
    out: u16,
}

impl<'tty, W:Write> App<'tty, W> {
    fn new(core: Core<'tty,W>) -> Self {
        let code = String::from("nop\nput 17 gp1\nadd gp1 gp1\ncp ans out");
        let memory = [4096; 65_536];
        let lcd0: u16 = 65_432;
        let lcd1: u16 = 01_234;
        let out: u16 = 47;
        Self {
            core,
            code, memory,
            out, lcd0, lcd1
        }
    }
}

struct Layouts {
    code: Rect,
    lcd0: Rect,
    lcd1: Rect,
    memory: Rect,
    registers: Rect,
    special_registers: Rect,
    printer: Rect,
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
        let code = layout[0];

        let devices_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(7),
                Constraint::Length(7),
                Constraint::Fill(1),
            ])
            .split(layout[1]);
        let lcd0 = devices_layout[0];
        let lcd1 = devices_layout[1];
        let printer = devices_layout[2];

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
            lcd0,
            lcd1,
            memory,
            registers, 
            special_registers,
            printer,
        }
    }
}

fn render<'tty, W: Write>(app: &'tty App<W>, frame: &mut Frame) {
    let layouts = Layouts::new(frame);


    render_code(&app.code, layouts.code, frame, "Code");
    render_lcd(app.lcd0, layouts.lcd0, frame, "LCD0 (dvc 0)");
    render_lcd(app.lcd1, layouts.lcd1, frame, "LCD1 (dvc 1)");
    render_printer("text", layouts.printer, frame, "Printer (dvc 2)");

    let gp_registers = vec![
        ("gp0", app.core.register_file.gp0),
        ("gp1", app.core.register_file.gp1),
        ("gp2", app.core.register_file.gp2),
        ("gp3", app.core.register_file.gp3),
        ("gp4", app.core.register_file.gp4),
        ("gp5", app.core.register_file.gp5),
        ("gp6", app.core.register_file.gp6),
        ("gp7", app.core.register_file.gp7),
    ];
    render_registers(
        gp_registers,
        layouts.registers, 
        frame,
        "General Purpose Registers"
    );


    let sp_registers = vec![
        ("ans", app.core.register_file.ans),
        ("out", app.out),
        ("dvc", app.core.register_file.dvc),
        ("pc", app.core.register_file.pc),
    ];
    render_registers(
        sp_registers,
        layouts.special_registers,
        frame,
        "Special Purpose Registers"
    );


    // Memory
    render_memory(&app.memory, layouts.memory, frame, "Memory");
}

fn render_code(
    code: &str,
    area: Rect, 
    frame: &mut Frame,
    title: &str,
) {
    let paragraph = Paragraph::new(code)
        .block(common_block(title));
    frame.render_widget(paragraph, area);
}


fn render_lcd(
    value: u16,
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

    let value = format!("{:0>5}", value);
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
    memory: &[u16; 65_536],
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
                true => Style::default().bg(Color::Gray),
                false => Style::default().bg(Color::White),
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
        .highlight_symbol(">>")
        .block( common_block(title));
    frame.render_widget(table, area);

}

fn common_block(title: &str) -> Block {
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().blue())
}
