use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, ConnectionStatus, Message, MessageRole, ToolStatus};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Messages
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Status bar
        ])
        .split(f.size());

    draw_header(f, app, chunks[0]);
    draw_messages(f, app, chunks[1]);
    draw_input(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let status_color = match app.status {
        ConnectionStatus::Connected => Color::Green,
        ConnectionStatus::Disconnected => Color::Red,
        ConnectionStatus::Connecting => Color::Yellow,
        ConnectionStatus::Error => Color::Red,
    };

    let header_text = Line::from(vec![
        Span::styled(" 🤖 ", Style::default()),
        Span::styled(
            "Copilot Agent TUI",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::styled("  |  ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", app.status),
            Style::default().fg(status_color),
        ),
        if app.is_streaming {
            Span::styled("  ◐ Streaming...", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]);

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .alignment(Alignment::Left);

    f.render_widget(header, area);
}

fn draw_messages(f: &mut Frame, app: &App, area: Rect) {
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .enumerate()
        .map(|(idx, msg)| {
            let item = format_message(msg, idx == app.messages.len() - 1 && msg.is_streaming);
            ListItem::new(item)
        })
        .collect();

    let title = if app.session_id.is_some() {
        format!("Messages (Session: {}...)", &app.session_id.as_ref().unwrap()[..8.min(app.session_id.as_ref().unwrap().len())])
    } else {
        "Messages".to_string()
    };

    let messages_list = List::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

    // Calculate visible range based on scroll offset
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
    let total_messages = app.messages.len();
    
    let start_idx = if total_messages > visible_height {
        total_messages.saturating_sub(visible_height).saturating_sub(app.scroll_offset)
    } else {
        0
    };
    
    let _end_idx = (start_idx + visible_height).min(total_messages);
    
    f.render_widget(messages_list, area);
}

fn format_message(msg: &Message, is_currently_streaming: bool) -> Text {
    let (prefix, style) = match msg.role {
        MessageRole::User => ("👤 ", Style::default().fg(Color::Cyan)),
        MessageRole::Assistant => ("🤖 ", Style::default().fg(Color::Green)),
        MessageRole::System => ("⚙️ ", Style::default().fg(Color::Yellow)),
    };

    let mut lines = vec![];

    // Main message line
    let timestamp = msg.timestamp.format("%H:%M:%S").to_string();
    let mut main_line = Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(&msg.content, style),
    ]);
    
    if is_currently_streaming {
        main_line.spans.push(Span::styled("▌", Style::default().fg(Color::Green)));
    }
    
    lines.push(main_line);

    // Tool calls
    for tool in &msg.tool_calls {
        let tool_style = match tool.status {
            ToolStatus::Running => Style::default().fg(Color::Yellow),
            ToolStatus::Success => Style::default().fg(Color::Green),
            ToolStatus::Error => Style::default().fg(Color::Red),
        };

        let tool_line = if let Some(ref result) = tool.result {
            let preview = if result.len() > 50 {
                format!("{}...", &result[..50])
            } else {
                result.clone()
            };
            Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{} ", tool.status), tool_style),
                Span::styled(format!("{}: ", tool.name), Style::default().fg(Color::Gray)),
                Span::styled(preview, tool_style),
            ])
        } else {
            Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{} ", tool.status), tool_style),
                Span::styled(format!("{}: Running...", tool.name), tool_style),
            ])
        };
        lines.push(tool_line);
    }

    // Timestamp
    lines.push(Line::from(vec![
        Span::styled(
            format!("   └─ {} ", timestamp),
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        ),
    ]));

    // Empty line for separation
    lines.push(Line::from(""));

    Text::from(lines)
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let input_style = Style::default().fg(Color::White);

    let input_text = if app.input.is_empty() && !app.is_streaming {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Green)),
            Span::styled(
                "Type a message and press Enter to send...",
                Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
            ),
        ])
    } else if app.is_streaming {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Streaming... Press Ctrl+S to stop",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Green)),
            Span::styled(&app.input, input_style),
            Span::styled("▌", Style::default().fg(Color::Green)),
        ])
    };

    let input = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input")
                .border_style(Style::default().fg(Color::Blue)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(input, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.is_streaming {
        "[Ctrl+S] Stop  [Ctrl+C] Quit"
    } else {
        "[Enter] Send  [Ctrl+N] New Session  [Ctrl+L] Clear  [Ctrl+C] Quit"
    };

    let status = format!(
        " Messages: {} | Scroll: {} | {}",
        app.messages.len(),
        app.scroll_offset,
        help_text
    );

    let status_bar = Paragraph::new(status)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray).add_modifier(Modifier::REVERSED));

    f.render_widget(status_bar, area);
}
