use crate::ai::agent_conversations_model::{
    AgentConversationEntry, AgentConversationEntryId, AgentRunDisplayStatus,
};
use crate::appearance::Appearance;
use crate::ui_components::icons::Icon;
use crate::util::time_format::format_approx_duration_from_now_utc;
use crate::workspace::view::local_agents::view::LocalAgentsViewAction;
use warpui::elements::{
    Border, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, DispatchEventResult,
    Element, EventHandler, Flex, Hoverable, MainAxisAlignment, MainAxisSize, MouseInBehavior,
    MouseStateHandle, Padding, ParentElement, Radius, Shrinkable, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::platform::Cursor;
use warpui::{AppContext, SingletonEntity};

const ROW_MIN_HEIGHT: f32 = 36.;
const ICON_SIZE: f32 = 14.;

#[derive(Clone, Default)]
pub struct ItemState {
    pub mouse_state: MouseStateHandle,
}

pub struct ItemProps<'a> {
    pub entry: &'a AgentConversationEntry,
    pub id: AgentConversationEntryId,
    pub index: usize,
    pub is_selected: bool,
    pub is_focused: bool,
    pub state: &'a ItemState,
}

pub struct NewItemProps<'a> {
    pub index: usize,
    pub is_selected: bool,
    pub state: &'a ItemState,
}

pub fn render_new_item(props: NewItemProps<'_>, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();
    let icon_color = theme.main_text_color(theme.background());

    let row = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(8.)
        .with_child(
            ConstrainedBox::new(Icon::Plus.to_warpui_icon(icon_color).finish())
                .with_width(ICON_SIZE)
                .with_height(ICON_SIZE)
                .finish(),
        )
        .with_child(
            Text::new_inline("New local agent", appearance.ui_font_family(), 13.)
                .with_color(icon_color.into())
                .finish(),
        )
        .finish();

    EventHandler::new(
        Hoverable::new(props.state.mouse_state.clone(), move |mouse_state| {
            row_container(row, props.is_selected || mouse_state.is_hovered(), app)
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(|ctx, _, _| {
            ctx.dispatch_typed_action(LocalAgentsViewAction::NewLocalAgentInNewTab);
        })
        .finish(),
    )
    .on_mouse_in(
        move |ctx, _, _| {
            ctx.dispatch_typed_action(LocalAgentsViewAction::SetSelectedIndex(props.index));
            DispatchEventResult::PropagateToParent
        },
        Some(MouseInBehavior {
            fire_on_synthetic_events: false,
            fire_when_covered: true,
        }),
    )
    .finish()
}

pub fn render_item(props: ItemProps<'_>, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();
    let title_color = if props.is_focused {
        theme.active_ui_text_color()
    } else {
        theme.main_text_color(theme.background())
    };

    let title = Text::new_inline(
        props.entry.display.title.clone(),
        appearance.ui_font_family(),
        13.,
    )
    .with_color(title_color.into())
    .with_style(Properties::default().weight(if props.is_focused {
        Weight::Semibold
    } else {
        Weight::Normal
    }))
    .finish();

    let timestamp = Text::new_inline(
        format_approx_duration_from_now_utc(props.entry.display.last_updated),
        appearance.ui_font_family(),
        11.,
    )
    .with_color(theme.sub_text_color(theme.background()).into())
    .finish();

    let status = status_label(&props.entry.display.status);
    let subtitle = Text::new_inline(status, appearance.ui_font_family(), 11.)
        .with_color(theme.sub_text_color(theme.background()).into())
        .finish();

    let text_column = Shrinkable::new(
        1.,
        Flex::column()
            .with_spacing(1.)
            .with_child(title)
            .with_child(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                    .with_child(Shrinkable::new(1., subtitle).finish())
                    .with_child(timestamp)
                    .finish(),
            )
            .finish(),
    )
    .finish();

    let row = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(8.)
        .with_child(status_icon(&props.entry.display.status, app))
        .with_child(text_column)
        .finish();

    EventHandler::new(
        Hoverable::new(props.state.mouse_state.clone(), move |mouse_state| {
            row_container(
                row,
                props.is_selected || props.is_focused || mouse_state.is_hovered(),
                app,
            )
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(LocalAgentsViewAction::OpenConversation { id: props.id });
        })
        .finish(),
    )
    .on_mouse_in(
        move |ctx, _, _| {
            ctx.dispatch_typed_action(LocalAgentsViewAction::SetSelectedIndex(props.index));
            DispatchEventResult::PropagateToParent
        },
        Some(MouseInBehavior {
            fire_on_synthetic_events: false,
            fire_when_covered: true,
        }),
    )
    .finish()
}

fn row_container(row: Box<dyn Element>, highlighted: bool, app: &AppContext) -> Box<dyn Element> {
    let theme = Appearance::as_ref(app).theme();
    let mut container = Container::new(row)
        .with_padding(Padding::uniform(0.).with_left(12.).with_right(12.))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)));
    if highlighted {
        container = container.with_background(theme.surface_overlay_1());
    }

    ConstrainedBox::new(container.finish())
        .with_min_height(ROW_MIN_HEIGHT)
        .finish()
}

fn status_icon(status: &AgentRunDisplayStatus, app: &AppContext) -> Box<dyn Element> {
    let theme = Appearance::as_ref(app).theme();
    let (icon, color) = match status.status_filter() {
        crate::ai::agent_conversations_model::StatusFilter::Working => {
            (Icon::Circle, theme.ansi_fg_magenta().into())
        }
        crate::ai::agent_conversations_model::StatusFilter::Done => {
            (Icon::Check, theme.ansi_fg_green().into())
        }
        crate::ai::agent_conversations_model::StatusFilter::Failed => {
            (Icon::Triangle, theme.ansi_fg_red().into())
        }
        crate::ai::agent_conversations_model::StatusFilter::All => {
            (Icon::Terminal, theme.sub_text_color(theme.background()))
        }
    };

    ConstrainedBox::new(icon.to_warpui_icon(color).finish())
        .with_width(ICON_SIZE)
        .with_height(ICON_SIZE)
        .finish()
}

fn status_label(status: &AgentRunDisplayStatus) -> &'static str {
    match status.status_filter() {
        crate::ai::agent_conversations_model::StatusFilter::Working => "Active",
        crate::ai::agent_conversations_model::StatusFilter::Done => "Done",
        crate::ai::agent_conversations_model::StatusFilter::Failed => "Needs attention",
        crate::ai::agent_conversations_model::StatusFilter::All => "Local",
    }
}

pub fn render_section_header(label: &'static str, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();

    Container::new(
        Text::new_inline(label, appearance.ui_font_family(), 11.)
            .with_color(theme.sub_text_color(theme.background()).into())
            .with_style(Properties::default().weight(Weight::Semibold))
            .finish(),
    )
    .with_padding(Padding::uniform(0.).with_left(12.).with_right(12.))
    .with_border(Border::bottom(1.).with_border_fill(theme.surface_3()))
    .with_vertical_padding(7.)
    .finish()
}
