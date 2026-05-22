use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::{Arc, Mutex};

use crate::ai::active_agent_views_model::{ActiveAgentViewsModel, ConversationOrTaskId};
use crate::ai::agent_conversations_model::{
    AgentConversationEntryId, AgentConversationNavigationSubject, AgentConversationsModel,
};
use crate::appearance::Appearance;
use crate::editor::{
    EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys,
    PropagateHorizontalNavigationKeys, SingleLineEditorOptions, TextOptions,
};
use crate::workspace::view::local_agents::item::{
    render_item, render_new_item, render_section_header, ItemProps, ItemState, NewItemProps,
};
use warp_editor::editor::NavigationKey;
use warpui::elements::{
    Border, ChildView, Clipped, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment,
    Element, Fill, Flex, FormattedTextElement, Hoverable, MainAxisAlignment, MainAxisSize,
    MouseStateHandle, Padding, ParentElement, Radius, ScrollStateHandle, Scrollable,
    ScrollableElement, ScrollbarWidth, Shrinkable, Text, UniformList, UniformListState,
};
use warpui::fonts::{Properties, Weight};
use warpui::keymap::macros::*;
use warpui::keymap::FixedBinding;
use warpui::platform::Cursor;
use warpui::text_layout::TextAlignment;
use warpui::{
    AppContext, BlurContext, Entity, ModelHandle, SingletonEntity, TypedActionView, View,
    ViewContext, ViewHandle, WindowId,
};

use super::view_model::{LocalAgentEntry, LocalAgentsViewModel};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LocalAgentsSection {
    Active,
    Past,
}

#[derive(Clone, Debug)]
enum ListItem {
    SectionHeader(LocalAgentsSection),
    LocalAgent(LocalAgentEntry),
    NewLocalAgent,
}

#[derive(Clone, Debug)]
pub enum LocalAgentsViewAction {
    OpenConversation { id: AgentConversationEntryId },
    NewLocalAgentInNewTab,
    ArrowUp,
    ArrowDown,
    Enter,
    SetSelectedIndex(usize),
    ClearSelectedIndex,
}

pub enum Event {
    NewLocalAgentInNewTab,
    OpenConversation { id: AgentConversationEntryId },
    DeleteConversation { id: AgentConversationEntryId },
}

#[derive(Default)]
struct StateHandles {
    list_state: UniformListState,
    scroll_state: ScrollStateHandle,
    item_states: HashMap<AgentConversationEntryId, ItemState>,
    new_item_state: ItemState,
    list_hover: MouseStateHandle,
    zero_state_button: MouseStateHandle,
}

pub struct LocalAgentsView {
    window_id: WindowId,
    view_model: ModelHandle<LocalAgentsViewModel>,
    query_editor: ViewHandle<EditorView>,
    selected_index: Option<usize>,
    list_items: Arc<Vec<ListItem>>,
    state_handles: StateHandles,
}

pub fn register_local_agents_view_bindings(app: &mut AppContext) {
    app.register_fixed_bindings([
        FixedBinding::new(
            "up",
            LocalAgentsViewAction::ArrowUp,
            id!(LocalAgentsView::ui_name()),
        ),
        FixedBinding::new(
            "down",
            LocalAgentsViewAction::ArrowDown,
            id!(LocalAgentsView::ui_name()),
        ),
        FixedBinding::new(
            "enter",
            LocalAgentsViewAction::Enter,
            id!(LocalAgentsView::ui_name()),
        ),
    ]);
}

impl LocalAgentsView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let view_model = ctx.add_model(LocalAgentsViewModel::new);

        ctx.subscribe_to_model(&view_model, |me, _, _, ctx| {
            me.sync_list_items(ctx);
        });

        let active_agent_views_model = ActiveAgentViewsModel::handle(ctx);
        ctx.subscribe_to_model(&active_agent_views_model, |me, _, _, ctx| {
            me.sync_list_items(ctx);
        });

        let query_editor = ctx.add_typed_action_view(|ctx| {
            let appearance = Appearance::as_ref(ctx);
            let mut editor = EditorView::single_line(
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(13.), appearance),
                    select_all_on_focus: true,
                    clear_selections_on_blur: true,
                    propagate_and_no_op_vertical_navigation_keys:
                        PropagateAndNoOpNavigationKeys::Always,
                    propagate_horizontal_navigation_keys: PropagateHorizontalNavigationKeys::Always,
                    ..Default::default()
                },
                ctx,
            );
            editor.set_placeholder_text("Search local agents", ctx);
            editor
        });
        ctx.subscribe_to_view(&query_editor, |me, _, event, ctx| {
            me.handle_query_editor_event(event, ctx);
        });

        let mut view = Self {
            window_id: ctx.window_id(),
            view_model,
            query_editor,
            selected_index: None,
            list_items: Arc::new(Vec::new()),
            state_handles: StateHandles {
                list_state: UniformListState::new(),
                scroll_state: Arc::new(Mutex::new(Default::default())),
                ..Default::default()
            },
        };
        view.sync_list_items(ctx);
        view
    }

    pub fn on_left_panel_focused(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.query_editor);
        let focused_conversation =
            ActiveAgentViewsModel::as_ref(ctx).get_focused_conversation(ctx.window_id());
        self.selected_index = focused_conversation
            .map(AgentConversationEntryId::from)
            .and_then(|id| self.index_of_entry(id));
        ctx.notify();
    }

    fn handle_query_editor_event(&mut self, event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        match event {
            EditorEvent::Edited(_) => {
                let query = self.query_editor.as_ref(ctx).buffer_text(ctx);
                self.view_model.update(ctx, |model, ctx| {
                    model.set_search_query(query, ctx);
                });
            }
            EditorEvent::Navigate(NavigationKey::Down) => self.move_selection_down(ctx),
            EditorEvent::Navigate(NavigationKey::Up) => self.move_selection_up(ctx),
            EditorEvent::Enter => self.activate_selected_item(ctx),
            _ => {}
        }
    }

    fn sync_list_items(&mut self, ctx: &mut ViewContext<Self>) {
        let model = self.view_model.as_ref(ctx);
        let current_ids: HashSet<_> = model.current_ids().copied().collect();
        self.state_handles
            .item_states
            .retain(|id, _| current_ids.contains(id));
        for id in &current_ids {
            self.state_handles.item_states.entry(*id).or_default();
        }

        self.rebuild_list_items(ctx);
        if self
            .selected_index
            .is_some_and(|index| index >= self.list_items.len() || !self.is_selectable(index))
        {
            self.selected_index = None;
        }
        ctx.notify();
    }

    fn rebuild_list_items(&mut self, ctx: &mut ViewContext<Self>) {
        let active_views_model = ActiveAgentViewsModel::as_ref(ctx);
        let active_ids: HashSet<_> = active_views_model
            .get_all_open_conversation_ids(ctx)
            .into_iter()
            .map(AgentConversationEntryId::from)
            .collect();
        let focused_id = active_views_model
            .get_focused_conversation(self.window_id)
            .map(AgentConversationEntryId::from);
        let model = self.view_model.as_ref(ctx);

        let mut active_items = Vec::new();
        let mut past_items = Vec::new();
        for entry in model.filtered_items() {
            let local_conversation_entry_id = model
                .get_item_by_id(&entry.id, ctx)
                .and_then(|entry| entry.identity.local_conversation_id)
                .map(AgentConversationEntryId::Conversation);
            let is_active = active_ids.contains(&entry.id)
                || local_conversation_entry_id.is_some_and(|id| active_ids.contains(&id))
                || focused_id
                    .is_some_and(|id| id == entry.id || Some(id) == local_conversation_entry_id);

            if is_active {
                active_items.push(ListItem::LocalAgent(entry.clone()));
            } else {
                past_items.push(ListItem::LocalAgent(entry.clone()));
            }
        }

        active_items.sort_by(|a, b| {
            self.last_opened_time(b, ctx)
                .cmp(&self.last_opened_time(a, ctx))
        });

        let mut items = Vec::new();
        if !active_items.is_empty() {
            items.push(ListItem::SectionHeader(LocalAgentsSection::Active));
            items.extend(active_items);
        }
        items.push(ListItem::NewLocalAgent);
        if !past_items.is_empty() {
            items.push(ListItem::SectionHeader(LocalAgentsSection::Past));
            items.extend(past_items);
        }
        self.list_items = Arc::new(items);
    }

    fn last_opened_time(
        &self,
        item: &ListItem,
        ctx: &AppContext,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        let ListItem::LocalAgent(entry) = item else {
            return None;
        };
        let active_views_model = ActiveAgentViewsModel::as_ref(ctx);
        let model = self.view_model.as_ref(ctx);
        let entry_time =
            active_views_model.get_last_opened_time(&ConversationOrTaskId::from(entry.id));
        let local_time = model
            .get_item_by_id(&entry.id, ctx)
            .and_then(|item| item.identity.local_conversation_id)
            .and_then(|id| {
                active_views_model.get_last_opened_time(&ConversationOrTaskId::ConversationId(id))
            });
        entry_time.max(local_time)
    }

    fn index_of_entry(&self, id: AgentConversationEntryId) -> Option<usize> {
        self.list_items.iter().position(|item| match item {
            ListItem::LocalAgent(entry) => entry.id == id,
            ListItem::SectionHeader(_) | ListItem::NewLocalAgent => false,
        })
    }

    fn is_selectable(&self, index: usize) -> bool {
        self.list_items
            .get(index)
            .is_some_and(|item| matches!(item, ListItem::LocalAgent(_) | ListItem::NewLocalAgent))
    }

    fn move_selection_up(&mut self, ctx: &mut ViewContext<Self>) {
        let start = self.selected_index.unwrap_or(self.list_items.len());
        for index in (0..start).rev() {
            if self.is_selectable(index) {
                self.selected_index = Some(index);
                ctx.notify();
                return;
            }
        }
        self.selected_index = None;
        ctx.focus(&self.query_editor);
        ctx.notify();
    }

    fn move_selection_down(&mut self, ctx: &mut ViewContext<Self>) {
        let start = self.selected_index.map(|index| index + 1).unwrap_or(0);
        for index in start..self.list_items.len() {
            if self.is_selectable(index) {
                self.selected_index = Some(index);
                ctx.focus_self();
                ctx.notify();
                return;
            }
        }
        self.selected_index = None;
        ctx.focus(&self.query_editor);
        ctx.notify();
    }

    fn activate_selected_item(&mut self, ctx: &mut ViewContext<Self>) {
        let Some(item) = self
            .selected_index
            .and_then(|index| self.list_items.get(index))
            .cloned()
        else {
            return;
        };

        match item {
            ListItem::NewLocalAgent => ctx.emit(Event::NewLocalAgentInNewTab),
            ListItem::LocalAgent(entry) => self.open_entry(entry.id, ctx),
            ListItem::SectionHeader(_) => {}
        }
    }

    fn open_entry(&self, id: AgentConversationEntryId, ctx: &mut ViewContext<Self>) {
        if let Some(action) = AgentConversationsModel::resolve_open_action(
            AgentConversationNavigationSubject::Entry(id),
            None,
            ctx,
        ) {
            ctx.dispatch_typed_action(&action);
        }
        ctx.emit(Event::OpenConversation { id });
    }
}

fn render_search_box(query_editor: &ViewHandle<EditorView>, app: &AppContext) -> Box<dyn Element> {
    let theme = Appearance::as_ref(app).theme();
    Container::new(
        Container::new(
            Shrinkable::new(
                1.,
                Clipped::new(ChildView::new(query_editor).finish()).finish(),
            )
            .finish(),
        )
        .with_padding(Padding::uniform(5.).with_left(10.).with_right(10.))
        .with_border(Border::all(1.).with_border_fill(theme.surface_3()))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
        .finish(),
    )
    .with_horizontal_padding(10.)
    .with_vertical_padding(6.)
    .finish()
}

fn render_zero_state(mouse_state: MouseStateHandle, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();

    let content = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(8.)
        .with_child(
            Text::new("No local agents", appearance.ui_font_family(), 13.)
                .with_color(theme.sub_text_color(theme.background()).into_solid())
                .with_style(Properties::default().weight(Weight::Semibold))
                .finish(),
        )
        .with_child(
            ConstrainedBox::new(
                FormattedTextElement::from_str(
                    "Local interactive agent conversations will appear here.",
                    appearance.ui_font_family(),
                    12.,
                )
                .with_alignment(TextAlignment::Center)
                .with_color(theme.disabled_ui_text_color().into_solid())
                .finish(),
            )
            .with_max_width(190.)
            .finish(),
        )
        .with_child(
            Hoverable::new(mouse_state, move |mouse_state| {
                let mut button = Container::new(
                    Text::new_inline("New local agent", appearance.ui_font_family(), 12.)
                        .with_color(theme.main_text_color(theme.background()).into_solid())
                        .finish(),
                )
                .with_horizontal_padding(8.)
                .with_vertical_padding(4.)
                .with_border(Border::all(1.).with_border_fill(theme.surface_3()))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)));
                if mouse_state.is_hovered() {
                    button = button.with_background(theme.surface_3());
                }
                button.finish()
            })
            .with_cursor(Cursor::PointingHand)
            .on_click(|ctx, _, _| {
                ctx.dispatch_typed_action(LocalAgentsViewAction::NewLocalAgentInNewTab);
            })
            .finish(),
        )
        .finish();

    Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_main_axis_alignment(MainAxisAlignment::Center)
        .with_child(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Max)
                .with_main_axis_alignment(MainAxisAlignment::Center)
                .with_child(content)
                .finish(),
        )
        .finish()
}

impl Entity for LocalAgentsView {
    type Event = Event;
}

impl TypedActionView for LocalAgentsView {
    type Action = LocalAgentsViewAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            LocalAgentsViewAction::OpenConversation { id } => self.open_entry(*id, ctx),
            LocalAgentsViewAction::NewLocalAgentInNewTab => {
                ctx.emit(Event::NewLocalAgentInNewTab);
            }
            LocalAgentsViewAction::ArrowUp => self.move_selection_up(ctx),
            LocalAgentsViewAction::ArrowDown => self.move_selection_down(ctx),
            LocalAgentsViewAction::Enter => self.activate_selected_item(ctx),
            LocalAgentsViewAction::SetSelectedIndex(index) => {
                self.selected_index = Some(*index);
                ctx.notify();
            }
            LocalAgentsViewAction::ClearSelectedIndex => {
                self.selected_index = None;
                ctx.notify();
            }
        }
    }
}

impl View for LocalAgentsView {
    fn ui_name() -> &'static str {
        "LocalAgentsView"
    }

    fn on_blur(&mut self, _: &BlurContext, ctx: &mut ViewContext<Self>) {
        if !ctx.is_self_or_child_focused() {
            self.selected_index = None;
            ctx.notify();
        }
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let view_model = self.view_model.as_ref(app);

        let content = if view_model.unfiltered_item_count() == 0 {
            render_zero_state(self.state_handles.zero_state_button.clone(), app)
        } else if self.list_items.is_empty() {
            Container::new(
                Text::new_inline(
                    "No matching local agents",
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_color(theme.sub_text_color(theme.background()).into())
                .finish(),
            )
            .with_horizontal_padding(12.)
            .with_vertical_padding(8.)
            .finish()
        } else {
            let model_handle = self.view_model.downgrade();
            let item_states = self.state_handles.item_states.clone();
            let new_item_state = self.state_handles.new_item_state.clone();
            let selected_index = self.selected_index;
            let focused_conversation = ActiveAgentViewsModel::as_ref(app)
                .get_focused_conversation(self.window_id)
                .map(AgentConversationEntryId::from);

            let list_items = self.list_items.clone();
            let list = UniformList::new(
                self.state_handles.list_state.clone(),
                self.list_items.len(),
                move |range: Range<usize>, app: &AppContext| {
                    let view_model = model_handle
                        .upgrade(app)
                        .expect("Model handle should be valid");
                    let view_model = view_model.as_ref(app);

                    range
                        .filter_map(|index| {
                            let item = list_items.get(index)?;
                            match item {
                                ListItem::SectionHeader(LocalAgentsSection::Active) => {
                                    Some(render_section_header("ACTIVE", app))
                                }
                                ListItem::SectionHeader(LocalAgentsSection::Past) => {
                                    Some(render_section_header("PAST", app))
                                }
                                ListItem::NewLocalAgent => Some(render_new_item(
                                    NewItemProps {
                                        index,
                                        is_selected: selected_index == Some(index),
                                        state: &new_item_state,
                                    },
                                    app,
                                )),
                                ListItem::LocalAgent(entry) => {
                                    let agent_entry = view_model.get_item_by_id(&entry.id, app)?;
                                    let local_conversation_entry_id = agent_entry
                                        .identity
                                        .local_conversation_id
                                        .map(AgentConversationEntryId::Conversation);
                                    let is_focused = focused_conversation.is_some_and(|focused| {
                                        focused == entry.id
                                            || Some(focused) == local_conversation_entry_id
                                    });
                                    let state = item_states.get(&entry.id)?;
                                    Some(render_item(
                                        ItemProps {
                                            entry: &agent_entry,
                                            id: entry.id,
                                            index,
                                            is_selected: selected_index == Some(index),
                                            is_focused,
                                            state,
                                        },
                                        app,
                                    ))
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                },
            )
            .finish_scrollable();

            let scrollable = Scrollable::vertical(
                self.state_handles.scroll_state.clone(),
                list,
                ScrollbarWidth::Auto,
                theme.nonactive_ui_detail().into(),
                theme.active_ui_detail().into(),
                Fill::None,
            )
            .with_overlayed_scrollbar()
            .finish();

            Hoverable::new(self.state_handles.list_hover.clone(), move |_| scrollable)
                .on_hover(|is_hovered, ctx, _, _| {
                    if !is_hovered {
                        ctx.dispatch_typed_action(LocalAgentsViewAction::ClearSelectedIndex);
                    }
                })
                .with_skip_synthetic_hover_out()
                .finish()
        };

        Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(render_search_box(&self.query_editor, app))
            .with_child(Shrinkable::new(1., content).finish())
            .finish()
    }
}
