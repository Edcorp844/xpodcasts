use podcasts_data::EpisodeId;

#[derive(Debug, Clone)]
pub struct PlayList {
    ids: Vec<EpisodeId>,
    current_index: Option<usize>,
}

impl Default for PlayList {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayList {
    pub fn new() -> Self {
        Self {
            ids: Vec::new(),
            current_index: None,
        }
    }

    /// Adds an episode to the end of the queue. No-op if it's already present.
    pub fn push_back(&mut self, id: EpisodeId) {
        if !self.ids.contains(&id) {
            self.ids.push(id);
        }
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    /// Replaces the whole queue and jumps to `starting_id` if found.
    /// If `starting_id` isn't in `ids`, falls back to the first item
    /// (or `None` if the new list is empty).
    pub fn set_sequence(&mut self, ids: Vec<EpisodeId>, starting_id: &EpisodeId) {
        self.ids = ids;
        self.current_index = self
            .ids
            .iter()
            .position(|id| id == starting_id)
            .or(if self.ids.is_empty() { None } else { Some(0) });
    }

    /// Navigates to the next episode ID if it exists
    pub fn next(&mut self) -> Option<EpisodeId> {
        let current = self.current_index?;
        if current + 1 < self.ids.len() {
            Some(self.ids[current + 1].clone())
        } else {
            None // Already at the end of the list
        }
    }

    /// Navigates to the previous episode ID if it exists
    pub fn prev(&mut self) -> Option<EpisodeId> {
        let current = self.current_index?;
        if current > 0 {
            Some(self.ids[current - 1].clone())
        } else {
            None
        }
    }

    /// Gets the current active episode ID
    pub fn current(&self) -> Option<EpisodeId> {
        let idx = self.current_index?;
        self.ids.get(idx).cloned()
    }

    /// Removes an episode from the queue, adjusting `current_index` so
    /// playback position stays correct relative to the remaining items.
    pub fn remove(&mut self, id: &EpisodeId) {
        if let Some(pos) = self.ids.iter().position(|x| x == id) {
            if self.current_index == Some(pos) {
                self.current_index = None;
            }
            self.ids.remove(pos);
        }
    }

    pub fn set_current(&mut self, id: EpisodeId) {
        println!(
            "set_current called with {id:?}, ids: {:?}, current before: {:?}",
            self.ids, self.current_index
        );
        if let Some(pos) = self.ids.iter().position(|x| *x == id) {
            self.current_index = Some(pos);
        } else {
            self.push_back(id);
            if let Some(pos) = self.ids.iter().position(|x| *x == id) {
                self.current_index = Some(pos);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn play_list(&self) -> (Vec<EpisodeId>, Option<usize>) {
        (self.ids.clone(), self.current_index)
    }
}
