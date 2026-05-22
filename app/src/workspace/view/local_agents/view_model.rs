use crate::ai::agent_conversations_model::{
    AgentConversationEntry, AgentConversationEntryId, AgentConversationProvenance,
    AgentConversationsModel, AgentConversationsModelEvent, AgentManagementFilters, ArtifactFilter,
    CreatedOnFilter, CreatorFilter, OwnerFilter, SourceFilter, StatusFilter,
};
use fuzzy_match::match_indices_case_insensitive;
use warpui::{AppContext, Entity, ModelContext, ModelHandle, SingletonEntity};

pub struct LocalAgentsViewModelEvent;

#[derive(Clone, Debug)]
pub struct LocalAgentEntry {
    pub id: AgentConversationEntryId,
    pub highlight_indices: Vec<usize>,
}

pub struct LocalAgentsViewModel {
    conversations_model: ModelHandle<AgentConversationsModel>,
    cached_entry_ids: Vec<AgentConversationEntryId>,
    filtered_items: Vec<LocalAgentEntry>,
    search_query: String,
}

impl Entity for LocalAgentsViewModel {
    type Event = LocalAgentsViewModelEvent;
}

impl LocalAgentsViewModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let conversations_model = AgentConversationsModel::handle(ctx);

        ctx.subscribe_to_model(&conversations_model, |me, event, ctx| match event {
            AgentConversationsModelEvent::ConversationsLoaded
            | AgentConversationsModelEvent::NewTasksReceived
            | AgentConversationsModelEvent::TasksUpdated => {
                me.refresh_cached_items(ctx);
            }
            AgentConversationsModelEvent::ConversationUpdated { .. } => {
                ctx.emit(LocalAgentsViewModelEvent);
            }
            AgentConversationsModelEvent::ConversationArtifactsUpdated { .. } => {}
        });

        let mut model = Self {
            conversations_model,
            cached_entry_ids: Vec::new(),
            filtered_items: Vec::new(),
            search_query: String::new(),
        };
        model.refresh_cached_items(ctx);
        model
    }

    fn refresh_cached_items(&mut self, ctx: &mut ModelContext<Self>) {
        let model = self.conversations_model.as_ref(ctx);
        self.cached_entry_ids = model
            .get_entries(
                &AgentManagementFilters {
                    owners: OwnerFilter::PersonalOnly,
                    status: StatusFilter::All,
                    source: SourceFilter::All,
                    created_on: CreatedOnFilter::All,
                    creator: CreatorFilter::All,
                    artifact: ArtifactFilter::All,
                    environment: Default::default(),
                    harness: Default::default(),
                },
                ctx,
            )
            .into_iter()
            .filter(is_local_entry)
            .filter(|entry| entry.capabilities.can_open)
            .map(|entry| entry.id)
            .collect();

        self.apply_search_filter(ctx);
        ctx.emit(LocalAgentsViewModelEvent);
    }

    pub fn set_search_query(&mut self, query: String, ctx: &mut ModelContext<Self>) {
        if query == self.search_query {
            return;
        }

        self.search_query = query;
        self.apply_search_filter(ctx);
        ctx.emit(LocalAgentsViewModelEvent);
    }

    fn apply_search_filter(&mut self, ctx: &mut ModelContext<Self>) {
        let search_query = self.search_query.trim().to_lowercase();
        let conversations_model = self.conversations_model.as_ref(ctx);

        if search_query.is_empty() {
            self.filtered_items = self
                .cached_entry_ids
                .iter()
                .map(|id| LocalAgentEntry {
                    id: *id,
                    highlight_indices: vec![],
                })
                .collect();
            return;
        }

        let mut matched_items: Vec<(i64, LocalAgentEntry)> = self
            .cached_entry_ids
            .iter()
            .filter_map(|id| {
                let item = conversations_model.get_entry_by_id(id, ctx)?;
                match_indices_case_insensitive(&item.display.title, &search_query).map(|result| {
                    (
                        result.score,
                        LocalAgentEntry {
                            id: *id,
                            highlight_indices: result.matched_indices,
                        },
                    )
                })
            })
            .collect();

        matched_items.sort_by(|a, b| b.0.cmp(&a.0));
        self.filtered_items = matched_items.into_iter().map(|(_, item)| item).collect();
    }

    pub fn unfiltered_item_count(&self) -> usize {
        self.cached_entry_ids.len()
    }

    pub fn filtered_items(&self) -> &[LocalAgentEntry] {
        &self.filtered_items
    }

    pub fn get_item_by_id(
        &self,
        id: &AgentConversationEntryId,
        ctx: &AppContext,
    ) -> Option<AgentConversationEntry> {
        self.conversations_model
            .as_ref(ctx)
            .get_entry_by_id(id, ctx)
    }

    pub fn current_ids(&self) -> impl Iterator<Item = &AgentConversationEntryId> {
        self.filtered_items.iter().map(|item| &item.id)
    }
}

fn is_local_entry(entry: &AgentConversationEntry) -> bool {
    entry.provenance == AgentConversationProvenance::LocalInteractive
        || entry.backing.has_local_persisted_data
        || entry.identity.local_conversation_id.is_some()
}
