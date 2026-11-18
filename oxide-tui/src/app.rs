use anyhow::Result;
use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use oxide_common::{
    protocol::{OpMessage, SrvMessage},
    types::*,
};
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;
use uuid::Uuid;

// ── Tab ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Dashboard,
    Agents,
    Listeners,
    Sessions,
    Loot,
    Build,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Dashboard, Tab::Agents, Tab::Listeners, Tab::Sessions, Tab::Loot, Tab::Build]
    }
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => " [1] Dashboard ",
            Tab::Agents    => " [2] Agents ",
            Tab::Listeners => " [3] Listeners ",
            Tab::Sessions  => " [4] Sessions ",
            Tab::Loot      => " [5] Loot ",
            Tab::Build     => " [6] Build ",
        }
    }
    pub fn index(&self) -> usize {
        Tab::all().iter().position(|t| t == self).unwrap_or(0)
    }
}

// ── Modal ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Modal {
    NewListener(ListenerDraft),
    GenPayload(PayloadDraft),
    AgentInteract { agent_id: Uuid, input: String },
    Confirm { message: String, action: String },
}

#[derive(Debug, Clone, Default)]
pub struct ListenerDraft {
    pub focused:  usize,
    pub name:     String,
    pub protocol: usize,
    pub host:     String,
    pub port:     String,
    pub domain:   String,
}

impl ListenerDraft {
    pub fn protocols() -> &'static [&'static str] {
        &["http", "https", "tcp", "dns", "github", "discord"]
    }
}

#[derive(Debug, Clone)]
pub struct PayloadDraft {
    pub focused:        usize,
    pub listener_idx:   usize,
    pub format_idx:     usize,
    pub sleep_ms:       String,
    pub jitter_pct:     String,
    pub output_path:    String,
    pub sleep_obf_idx:  usize,
    pub injection_idx:  usize,
    pub amsi:           bool,
    pub etw:            bool,
    pub sandbox:        bool,
    pub syscalls:       bool,
    pub stack_spoof:    bool,
    pub module_stomp:   bool,
    pub block_dlls:     bool,
    pub encrypted:      bool,
    pub dns_dga:        bool,
}

impl Default for PayloadDraft {
    fn default() -> Self {
        PayloadDraft {
            focused:       0,
            listener_idx:  0,
            format_idx:    0,
            sleep_ms:      "5000".into(),
            jitter_pct:    "10".into(),
            output_path:   "/tmp/agent.exe".into(),
            sleep_obf_idx: 0,
            injection_idx: 0,
            amsi:          true,
            etw:           true,
            sandbox:       false,
            syscalls:      false,
            stack_spoof:   false,
            module_stomp:  false,
            block_dlls:    false,
            encrypted:     true,
            dns_dga:       false,
        }
    }
}

impl PayloadDraft {
    pub const FIELD_COUNT: usize = 16;
    pub fn formats()    -> &'static [&'static str] { &["exe", "dll", "shellcode"] }
    pub fn sleep_obfs() -> &'static [&'static str] { &["none", "ekko", "foliage", "zilean"] }
    pub fn injections() -> &'static [&'static str] { &["CreateThread", "NtCreateThreadEx", "ApcQueue"] }

    pub fn to_build_options(&self, listeners: &[ListenerInfo]) -> BuildOptions {
        let listener_id = listeners.get(self.listener_idx)
            .map(|l| l.config.id)
            .unwrap_or_else(Uuid::nil);

        let format = match self.format_idx {
            1 => PayloadFormat::Dll,
            2 => PayloadFormat::Shellcode,
            _ => PayloadFormat::Exe,
        };
        let sleep_obf = match self.sleep_obf_idx {
            1 => SleepObf::Ekko,
            2 => SleepObf::Foliage,
            3 => SleepObf::Zilean,
            _ => SleepObf::None,
        };
        let injection = match self.injection_idx {
            1 => InjectionMethod::NtCreateThreadEx,
            2 => InjectionMethod::ApcQueue,
            _ => InjectionMethod::CreateThread,
        };

        BuildOptions {
            listener_id,
            format,
            sleep_ms:        self.sleep_ms.parse().unwrap_or(5000),
            jitter_pct:      self.jitter_pct.parse().unwrap_or(10),
            output_path:     self.output_path.clone(),
            sleep_obf,
            injection,
            amsi_bypass:     self.amsi,
            etw_bypass:      self.etw,
            sandbox_evasion: self.sandbox,
            indirect_syscalls: self.syscalls,
            stack_spoof:     self.stack_spoof,
            module_stomp:    self.module_stomp,
            block_dlls:      self.block_dlls,
            encrypted_comms: self.encrypted,
            dns_dga:         self.dns_dga,
            server_pubkey:   String::new(),
            c2_host: listeners.get(self.listener_idx)
                .map(|l| l.config.host.clone())
                .unwrap_or_else(|| "127.0.0.1".into()),
            c2_port: listeners.get(self.listener_idx)
                .map(|l| l.config.port)
                .unwrap_or(8080),
            protocol: listeners.get(self.listener_idx)
                .map(|l| l.config.protocol.to_string())
                .unwrap_or_else(|| "http".into()),
        }
    }
}

// ── Network graph node ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id:    Option<Uuid>,
    pub label: String,
    pub x:     f64,
    pub y:     f64,
    pub kind:  NodeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Server,
    Agent { status: AgentStatus, priv_lvl: Privilege },
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub tab:                Tab,
    pub agents:             Vec<AgentInfo>,
    pub listeners:          Vec<ListenerInfo>,
    pub loot:               Vec<LootEntry>,
    pub log:                VecDeque<String>,
    pub modal:              Option<Modal>,
    pub graph_nodes:        Vec<GraphNode>,
    pub selected_agent:     Option<usize>,
    pub selected_listener:  Option<usize>,
    pub selected_loot:      Option<usize>,
    pub command_input:      String,
    pub build_status:       Option<String>,
    pub ws_tx:              mpsc::Sender<OpMessage>,
    pub ws_connected:       bool,
    // Per-agent command/result history for the Sessions tab and interact modal
    pub agent_history:      HashMap<Uuid, VecDeque<String>>,
    // Interact modal scroll: 0 = bottom (newest), higher = scrolled up (older)
    pub interact_scroll:    usize,
    // Command history for ↑/↓ in interact modal
    pub cmd_history:        Vec<String>,
    pub cmd_history_idx:    Option<usize>,
    // task_id → agent_id for routing TaskResult to per-agent history
    pub pending_cmds:       HashMap<Uuid, String>,
    pub pending_task_agents: HashMap<Uuid, Uuid>,
    // Filter string (press '/' to activate)
    pub filter:             String,
    pub filter_active:      bool,
    pub beacon_history:     VecDeque<u64>,
    pub tick_count:         u64,
    pub beacons_this_sec:   u64,
}

impl App {
    pub fn new(ws_tx: mpsc::Sender<OpMessage>) -> Self {
        App {
            tab: Tab::Agents,
            agents: Vec::new(),
            listeners: Vec::new(),
            loot: Vec::new(),
            log: VecDeque::with_capacity(500),
            modal: None,
            graph_nodes: vec![GraphNode {
                id: None, label: "OXIDE C2".into(),
                x: 50.0, y: 50.0,
                kind: NodeKind::Server,
            }],
            selected_agent:    None,
            selected_listener: None,
            selected_loot:     None,
            command_input:     String::new(),
            build_status:      None,
            ws_tx,
            ws_connected:         false,
            agent_history:        HashMap::new(),
            interact_scroll:      0,
            cmd_history:          Vec::new(),
            cmd_history_idx:      None,
            pending_cmds:         HashMap::new(),
            pending_task_agents:  HashMap::new(),
            filter:               String::new(),
            filter_active:        false,
            beacon_history:       VecDeque::with_capacity(60),
            tick_count:           0,
            beacons_this_sec:     0,
        }
    }

    pub async fn send_op(&self, op: OpMessage) {
        if self.ws_tx.send(op).await.is_err() {
            // Channel closed — WS died; reconnect loop in main.rs will handle it
        }
    }

    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        if self.tick_count % 2 == 0 {
            if self.beacon_history.len() >= 60 {
                self.beacon_history.pop_front();
            }
            self.beacon_history.push_back(self.beacons_this_sec);
            self.beacons_this_sec = 0;
        }
    }

    pub fn push_log(&mut self, msg: impl Into<String>) {
        let now = chrono::Local::now().format("%H:%M:%S");
        self.log.push_front(format!("[{now}] {}", msg.into()));
        if self.log.len() > 500 { self.log.pop_back(); }
    }

    fn push_agent_hist(&mut self, agent_id: Uuid, msg: impl Into<String>) {
        let now = chrono::Local::now().format("%H:%M:%S");
        let hist = self.agent_history.entry(agent_id).or_default();
        hist.push_back(format!("[{now}] {}", msg.into()));
        if hist.len() > 1000 { hist.pop_front(); }
    }

    pub fn handle_server_message(&mut self, msg: SrvMessage) {
        match msg {
            SrvMessage::Agents { list } => {
                self.agents = list;
                self.rebuild_graph();
                self.push_log(format!("agents: {}", self.agents.len()));
            }
            SrvMessage::Listeners { list } => {
                self.listeners = list;
                self.push_log(format!("listeners: {}", self.listeners.len()));
            }
            SrvMessage::Loot { entries } => {
                self.loot = entries;
            }
            SrvMessage::AgentNew { agent } => {
                let id = agent.id;
                let msg = format!("NEW AGENT: {} @ {} ({})", agent.hostname, agent.ip, agent.privilege);
                self.push_log(msg.clone());
                self.push_agent_hist(id, format!("● Connected — {}", agent.hostname));
                self.agents.push(agent);
                self.rebuild_graph();
            }
            SrvMessage::AgentUpdate { agent } => {
                self.beacons_this_sec += 1;
                if let Some(a) = self.agents.iter_mut().find(|a| a.id == agent.id) {
                    *a = agent;
                }
                self.rebuild_graph();
            }
            SrvMessage::AgentDead { agent_id } => {
                self.push_agent_hist(agent_id, "○ Agent lost contact");
                self.agents.retain(|a| a.id != agent_id);
                self.rebuild_graph();
                self.push_log(format!("agent dead: {}", agent_id));
            }
            SrvMessage::TaskResult { result } => {
                self.push_log(format!(
                    "← result task={} agent={} out={}b err={:?}",
                    result.task_id, result.agent_id, result.output.len(), result.error
                ));
                let agent_id = if result.agent_id != Uuid::nil() {
                    result.agent_id
                } else {
                    self.pending_task_agents.get(&result.task_id)
                        .copied()
                        .unwrap_or(Uuid::nil())
                };
                self.pending_task_agents.remove(&result.task_id);

                let cmd = self.pending_cmds.remove(&result.task_id)
                    .unwrap_or_else(|| result.task_id.to_string());

                // Push output lines to per-agent history
                if !result.output.trim().is_empty() {
                    for line in result.output.trim().lines() {
                        self.push_agent_hist(agent_id, format!("  {line}"));
                        self.push_log(format!("  {line}"));
                    }
                }
                if let Some(ref e) = result.error {
                    self.push_agent_hist(agent_id, format!("[ERR] {e}"));
                    self.push_log(format!("  [ERR] {e}"));
                } else if result.output.trim().is_empty() {
                    self.push_agent_hist(agent_id, format!("← {cmd} (no output)"));
                    self.push_log(format!("← {cmd} (no output)"));
                } else {
                    self.push_agent_hist(agent_id, format!("← {cmd}"));
                    self.push_log(format!("← {cmd}"));
                }
            }
            SrvMessage::BuildStart { build_id } => {
                self.build_status = Some(format!("Building {build_id}…"));
                self.push_log(format!("build started: {build_id}"));
            }
            SrvMessage::BuildDone { build_id: _, path, size } => {
                self.build_status = Some(format!("Done: {path} ({size} bytes)"));
                self.push_log(format!("build done: {path}"));
            }
            SrvMessage::BuildFail { build_id: _, error } => {
                self.build_status = Some(format!("FAILED: {error}"));
                self.push_log(format!("build FAILED: {error}"));
            }
            SrvMessage::Log { level, message } => {
                self.push_log(format!("[{level}] {message}"));
            }
            SrvMessage::Error { message } => {
                self.push_log(format!("SERVER ERROR: {message}"));
            }
            _ => {}
        }
    }

    fn rebuild_graph(&mut self) {
        use std::f64::consts::PI;
        self.graph_nodes.clear();
        self.graph_nodes.push(GraphNode {
            id: None, label: "OXIDE C2".into(),
            x: 50.0, y: 50.0, kind: NodeKind::Server,
        });
        let n = self.agents.len();
        for (i, agent) in self.agents.iter().enumerate() {
            let angle = (i as f64 / n.max(1) as f64) * 2.0 * PI - PI / 2.0;
            let r = 35.0;
            self.graph_nodes.push(GraphNode {
                id: Some(agent.id),
                label: format!("{}\n{}", agent.hostname, agent.ip),
                x: 50.0 + r * angle.cos(),
                y: 50.0 + r * angle.sin(),
                kind: NodeKind::Agent {
                    status: agent.status.clone(),
                    priv_lvl: agent.privilege.clone(),
                },
            });
        }
    }

    pub async fn handle_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(key) => {
                if self.modal.is_none() {
                    // Filter mode: type to filter, Esc to clear
                    if self.filter_active {
                        match key.code {
                            KeyCode::Esc => {
                                self.filter_active = false;
                                self.filter.clear();
                            }
                            KeyCode::Backspace => { self.filter.pop(); }
                            KeyCode::Char(c) => { self.filter.push(c); }
                            KeyCode::Enter => { self.filter_active = false; }
                            _ => {}
                        }
                        return Ok(false);
                    }

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(true),
                        KeyCode::Tab => {
                            let idx = (self.tab.index() + 1) % Tab::all().len();
                            self.tab = Tab::all()[idx].clone();
                            self.filter.clear();
                            self.filter_active = false;
                        }
                        KeyCode::BackTab => {
                            let idx = (self.tab.index() + Tab::all().len() - 1) % Tab::all().len();
                            self.tab = Tab::all()[idx].clone();
                            self.filter.clear();
                            self.filter_active = false;
                        }
                        KeyCode::Char('1') => { self.tab = Tab::Dashboard; self.filter.clear(); }
                        KeyCode::Char('2') => { self.tab = Tab::Agents;    self.filter.clear(); }
                        KeyCode::Char('3') => { self.tab = Tab::Listeners; self.filter.clear(); }
                        KeyCode::Char('4') => { self.tab = Tab::Sessions;  self.filter.clear(); }
                        KeyCode::Char('5') => { self.tab = Tab::Loot;      self.filter.clear(); }
                        KeyCode::Char('6') => { self.tab = Tab::Build;     self.filter.clear(); }
                        KeyCode::Char('/') => {
                            if matches!(self.tab, Tab::Agents | Tab::Listeners) {
                                self.filter_active = true;
                                self.filter.clear();
                            }
                        }
                        KeyCode::Char('l') => {
                            self.send_op(OpMessage::ListListeners).await;
                            self.send_op(OpMessage::ListAgents).await;
                        }
                        KeyCode::Char('n') if self.tab == Tab::Listeners => {
                            self.modal = Some(Modal::NewListener(ListenerDraft {
                                host: "0.0.0.0".into(),
                                port: "8080".into(),
                                name: "Listener-1".into(),
                                ..Default::default()
                            }));
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') if self.tab == Tab::Listeners => {
                            if let Some(idx) = self.selected_listener {
                                if let Some(l) = self.listeners.get(idx) {
                                    let id = l.config.id;
                                    let name = l.config.name.clone();
                                    self.push_log(format!("stopping listener: {name}"));
                                    self.send_op(OpMessage::StopListener { id }).await;
                                    self.send_op(OpMessage::ListListeners).await;
                                }
                            }
                        }
                        KeyCode::Char('g') | KeyCode::Char('G') if self.tab == Tab::Build => {
                            self.modal = Some(Modal::GenPayload(PayloadDraft::default()));
                        }
                        KeyCode::Enter if self.tab == Tab::Agents => {
                            if let Some(idx) = self.selected_agent {
                                if let Some(agent) = self.agents.get(idx) {
                                    let id = agent.id;
                                    self.interact_scroll = 0;
                                    self.cmd_history_idx = None;
                                    self.modal = Some(Modal::AgentInteract {
                                        agent_id: id,
                                        input: String::new(),
                                    });
                                }
                            }
                        }
                        KeyCode::Enter if self.tab == Tab::Sessions => {
                            if let Some(idx) = self.selected_agent {
                                if let Some(agent) = self.agents.get(idx) {
                                    let id = agent.id;
                                    self.interact_scroll = 0;
                                    self.cmd_history_idx = None;
                                    self.modal = Some(Modal::AgentInteract {
                                        agent_id: id,
                                        input: String::new(),
                                    });
                                }
                            }
                        }
                        KeyCode::Up   => self.list_up(),
                        KeyCode::Down => self.list_down(),
                        _ => {}
                    }
                } else {
                    self.handle_modal_key(key).await?;
                }
            }
            Event::Mouse(mouse) => {
                self.handle_mouse(mouse);
            }
            _ => {}
        }
        Ok(false)
    }

    fn list_up(&mut self) {
        match self.tab {
            Tab::Agents | Tab::Sessions => {
                self.selected_agent = Some(
                    self.selected_agent.map(|i| i.saturating_sub(1)).unwrap_or(0)
                );
            }
            Tab::Listeners => {
                self.selected_listener = Some(
                    self.selected_listener.map(|i| i.saturating_sub(1)).unwrap_or(0)
                );
            }
            Tab::Loot => {
                self.selected_loot = Some(
                    self.selected_loot.map(|i| i.saturating_sub(1)).unwrap_or(0)
                );
            }
            _ => {}
        }
    }

    fn list_down(&mut self) {
        match self.tab {
            Tab::Agents | Tab::Sessions => {
                let max = self.agents.len().saturating_sub(1);
                self.selected_agent = Some(
                    self.selected_agent.map(|i| (i + 1).min(max)).unwrap_or(0)
                );
            }
            Tab::Listeners => {
                let max = self.listeners.len().saturating_sub(1);
                self.selected_listener = Some(
                    self.selected_listener.map(|i| (i + 1).min(max)).unwrap_or(0)
                );
            }
            Tab::Loot => {
                let max = self.loot.len().saturating_sub(1);
                self.selected_loot = Some(
                    self.selected_loot.map(|i| (i + 1).min(max)).unwrap_or(0)
                );
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            let _x = mouse.column;
            let _y = mouse.row;
        }
    }

    async fn handle_modal_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        // Handle AgentInteract special keys (Up/Down/PgUp/PgDn) BEFORE the main match,
        // so we can access self.cmd_history without a conflicting borrow on self.modal.
        let is_interact = matches!(&self.modal, Some(Modal::AgentInteract { .. }));
        if is_interact {
            match key.code {
                KeyCode::Up => {
                    let len = self.cmd_history.len();
                    if len > 0 {
                        let new_idx = match self.cmd_history_idx {
                            None      => len - 1,
                            Some(0)   => 0,
                            Some(i)   => i - 1,
                        };
                        self.cmd_history_idx = Some(new_idx);
                        let new_cmd = self.cmd_history[new_idx].clone();
                        if let Some(Modal::AgentInteract { input, .. }) = &mut self.modal {
                            *input = new_cmd;
                        }
                    }
                    return Ok(());
                }
                KeyCode::Down => {
                    let len = self.cmd_history.len();
                    match self.cmd_history_idx {
                        None => {}
                        Some(i) if i + 1 >= len => {
                            self.cmd_history_idx = None;
                            if let Some(Modal::AgentInteract { input, .. }) = &mut self.modal {
                                input.clear();
                            }
                        }
                        Some(i) => {
                            let new_idx = i + 1;
                            self.cmd_history_idx = Some(new_idx);
                            let new_cmd = self.cmd_history[new_idx].clone();
                            if let Some(Modal::AgentInteract { input, .. }) = &mut self.modal {
                                *input = new_cmd;
                            }
                        }
                    }
                    return Ok(());
                }
                KeyCode::PageUp => {
                    self.interact_scroll = self.interact_scroll.saturating_add(5);
                    return Ok(());
                }
                KeyCode::PageDown => {
                    self.interact_scroll = self.interact_scroll.saturating_sub(5);
                    return Ok(());
                }
                _ => {}
            }
        }

        match &mut self.modal {
            Some(Modal::NewListener(draft)) => {
                match key.code {
                    KeyCode::Esc => { self.modal = None; }
                    KeyCode::Tab | KeyCode::Down => {
                        draft.focused = (draft.focused + 1) % 5;
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        draft.focused = (draft.focused + 5 - 1) % 5;
                    }
                    KeyCode::Left => {
                        if draft.focused == 1 {
                            draft.protocol = (draft.protocol + ListenerDraft::protocols().len() - 1)
                                % ListenerDraft::protocols().len();
                        }
                    }
                    KeyCode::Right => {
                        if draft.focused == 1 {
                            draft.protocol = (draft.protocol + 1) % ListenerDraft::protocols().len();
                        }
                    }
                    KeyCode::Backspace => {
                        match draft.focused {
                            0 => { draft.name.pop(); }
                            2 => { draft.host.pop(); }
                            3 => { draft.port.pop(); }
                            4 => { draft.domain.pop(); }
                            _ => {}
                        }
                    }
                    KeyCode::Char(c) => {
                        match draft.focused {
                            0 => draft.name.push(c),
                            2 => draft.host.push(c),
                            3 => draft.port.push(c),
                            4 => draft.domain.push(c),
                            _ => {}
                        }
                    }
                    KeyCode::Enter => {
                        let draft_clone = draft.clone();
                        self.modal = None;
                        self.create_listener(draft_clone).await;
                    }
                    _ => {}
                }
            }
            Some(Modal::GenPayload(draft)) => {
                match key.code {
                    KeyCode::Esc => { self.modal = None; }
                    KeyCode::Tab | KeyCode::Down => {
                        draft.focused = (draft.focused + 1) % PayloadDraft::FIELD_COUNT;
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        draft.focused = (draft.focused + PayloadDraft::FIELD_COUNT - 1)
                            % PayloadDraft::FIELD_COUNT;
                    }
                    KeyCode::Left => {
                        match draft.focused {
                            0 => { draft.listener_idx = (draft.listener_idx + self.listeners.len().max(1) - 1) % self.listeners.len().max(1); }
                            1 => { draft.format_idx   = (draft.format_idx + 2) % 3; }
                            5 => { draft.sleep_obf_idx = (draft.sleep_obf_idx + 3) % 4; }
                            6 => { draft.injection_idx = (draft.injection_idx + 2) % 3; }
                            _ => {}
                        }
                    }
                    KeyCode::Right => {
                        match draft.focused {
                            0 => { draft.listener_idx = (draft.listener_idx + 1) % self.listeners.len().max(1); }
                            1 => { draft.format_idx   = (draft.format_idx + 1) % 3; }
                            5 => { draft.sleep_obf_idx = (draft.sleep_obf_idx + 1) % 4; }
                            6 => { draft.injection_idx = (draft.injection_idx + 1) % 3; }
                            _ => {}
                        }
                    }
                    KeyCode::Char(' ') => {
                        match draft.focused {
                            7  => draft.amsi        = !draft.amsi,
                            8  => draft.etw         = !draft.etw,
                            9  => draft.sandbox     = !draft.sandbox,
                            10 => draft.syscalls    = !draft.syscalls,
                            11 => draft.stack_spoof = !draft.stack_spoof,
                            12 => draft.module_stomp = !draft.module_stomp,
                            13 => draft.block_dlls  = !draft.block_dlls,
                            14 => draft.encrypted   = !draft.encrypted,
                            15 => draft.dns_dga     = !draft.dns_dga,
                            _  => {}
                        }
                    }
                    KeyCode::Backspace => {
                        match draft.focused {
                            2 => { draft.sleep_ms.pop(); }
                            3 => { draft.jitter_pct.pop(); }
                            4 => { draft.output_path.pop(); }
                            _ => {}
                        }
                    }
                    KeyCode::Char(c) => {
                        match draft.focused {
                            2 if c.is_ascii_digit() => draft.sleep_ms.push(c),
                            3 if c.is_ascii_digit() => draft.jitter_pct.push(c),
                            4 => draft.output_path.push(c),
                            _ => {}
                        }
                    }
                    KeyCode::Enter => {
                        let opts = draft.to_build_options(&self.listeners);
                        self.modal = None;
                        self.send_op(OpMessage::GeneratePayload { options: opts }).await;
                        self.push_log("Build started…");
                    }
                    _ => {}
                }
            }
            Some(Modal::AgentInteract { agent_id, input }) => {
                let aid = *agent_id;
                match key.code {
                    KeyCode::Esc => { self.modal = None; }
                    KeyCode::Backspace => { input.pop(); }
                    KeyCode::Char(c) => {
                        // Reset history navigation on any character input
                        input.push(c);
                    }
                    KeyCode::Enter => {
                        let cmd_str = input.trim().to_string();
                        *input = String::new();
                        // drop borrow on self.modal fields before accessing cmd_history etc.
                        drop(input);
                        drop(agent_id);

                        if !cmd_str.is_empty() {
                            // Store in cmd history
                            if self.cmd_history.last().map(|s| s.as_str()) != Some(&cmd_str) {
                                self.cmd_history.push(cmd_str.clone());
                                if self.cmd_history.len() > 200 {
                                    self.cmd_history.remove(0);
                                }
                            }
                            self.cmd_history_idx = None;

                            let parts: Vec<String> = cmd_str.splitn(2, ' ')
                                .map(|s| s.to_string())
                                .collect();
                            let command = parts[0].clone();
                            let args = if parts.len() > 1 {
                                parts[1].split_whitespace().map(|s| s.to_string()).collect()
                            } else {
                                vec![]
                            };
                            let task_id = Uuid::new_v4();
                            let task = AgentTask { id: task_id, command, args };

                            // Track command for display and routing
                            self.pending_cmds.insert(task_id, cmd_str.clone());
                            self.pending_task_agents.insert(task_id, aid);

                            // Push sent command to per-agent history
                            self.push_agent_hist(aid, format!("→ {cmd_str}"));
                            self.push_log(format!("→ {cmd_str}"));

                            // Reset scroll to bottom so user sees the response
                            self.interact_scroll = 0;

                            self.send_op(OpMessage::TaskAgent { agent_id: aid, task }).await;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn create_listener(&mut self, draft: ListenerDraft) {
        let protocol_str = ListenerDraft::protocols()[draft.protocol];
        let protocol = match protocol_str {
            "https"   => ListenerProtocol::Https,
            "tcp"     => ListenerProtocol::Tcp,
            "dns"     => ListenerProtocol::Dns,
            "github"  => ListenerProtocol::Github,
            "discord" => ListenerProtocol::Discord,
            _         => ListenerProtocol::Http,
        };
        let cfg = ListenerConfig {
            id:         Uuid::new_v4(),
            name:       draft.name,
            protocol,
            host:       draft.host,
            port:       draft.port.parse().unwrap_or(8080),
            tls_cert:   None,
            tls_key:    None,
            dns_domain: if draft.domain.is_empty() { None } else { Some(draft.domain) },
            saas_token: None,
            saas_gist:  None,
        };
        self.push_log(format!("creating listener: {} {}:{}", cfg.name, cfg.host, cfg.port));
        self.send_op(OpMessage::CreateListener { config: cfg }).await;
        self.send_op(OpMessage::ListListeners).await;
    }
}
