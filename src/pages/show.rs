use std::{collections::HashMap, sync::Arc};

use adw::prelude::*;
use gst_play::PlayState;
use podcasts_data::{
    Episode, EpisodeId, Show, ShowId,
    dbqueries::{self},
    errors::DataError,
};
use relm4::{Component, ComponentParts, ComponentSender, prelude::*};
use uuid::Uuid;

use crate::{
    components::{
        episode_list_item::{EpisodeListItem, EpisodeListItemInput, EpisodeListItemOutput},
        play_button::{
            EpisodePlayingState, PlayButton, PlayButtonInitData, PlayButtonInput, PlayButtonOutput,
        },
    },
    pages::all_episodes::{AllEpisodesPage, AllEpisodesPageInput, AllEpisodesPageOutput},
    util::{
        cover_image::{ImageSize, fetch_cached_image},
        episode_description_parser,
    },
    workers::action_worker::worker::{Action, ActionResult},
};

#[derive(Debug)]
pub struct ShowPage {
    show: Option<Show>,
    latest_episode: Option<EpisodeId>,
    latest_play_button: Controller<PlayButton>,
    episodes: FactoryVecDeque<EpisodeListItem>,
    index_by_id: HashMap<EpisodeId, relm4::factory::DynamicIndex>,
    episode_count: usize,
    show_image_texture: Option<adw::gdk::Texture>,
    all_episodes_page: Controller<AllEpisodesPage>,
    task_id: Uuid,
}

#[derive(Debug)]
pub enum ShowPageInput {
    ActionFinished(Uuid, ActionResult),
    GetShow(ShowId),
    ShowGotten(Result<Show, DataError>),
    ImageDownloaded(Option<adw::gdk::Texture>),
    TogglePlayLatest,
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
    EpisodeDeleted(EpisodeId),
    LoadEpisodes(Vec<Episode>),
    GotoAllEpisodesPage,
}

#[derive(Debug)]
pub enum ShowPageOutput {
    Execute(Uuid, Action),
    TogglePlay(EpisodeId),
    NotifyError(String),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
}

#[derive(Debug)]
pub enum ShowPageCmdInput {
    DownloadImage(Option<adw::gdk::Texture>),
}

#[relm4::component(pub)]
impl Component for ShowPage {
    type Init = ShowId;
    type Input = ShowPageInput;
    type Output = ShowPageOutput;
    type CommandOutput = ShowPageCmdInput;

    view! {
        adw::NavigationPage {

            #[wrap(Some)]
            #[name="nav_view"]
            set_child =&adw::NavigationView {

                add=&adw::NavigationPage{

                    #[wrap(Some)]
                    set_child = &adw::ToolbarView {

                        add_top_bar = &adw::HeaderBar {
                            set_show_start_title_buttons: false,
                            set_show_end_title_buttons: false,
                        },

                        #[wrap(Some)]
                        set_content = &gtk::ScrolledWindow {
                            set_vexpand: true,
                            set_hscrollbar_policy: gtk::PolicyType::Never,

                            adw::Clamp {
                                set_maximum_size: 1100,
                                set_tightening_threshold: 900,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_margin_all: 32,
                                    set_spacing: 24,

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_spacing: 28,
                                        set_valign: gtk::Align::Start,

                                        // Cover Art Block Container
                                        gtk::Overlay {
                                            set_height_request: 350,
                                            set_width_request: 350,
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Start,

                                            #[wrap(Some)]
                                            set_child = &gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_hexpand: true,
                                                set_vexpand: true,
                                                set_halign: gtk::Align::Fill,
                                                set_valign: gtk::Align::Fill,

                                                inline_css: "
                                                    background-color: mix(@window_bg_color, @card_fg_color, 0.1);
                                                    border-radius: 16px;
                                                    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32);
                                                    border: 1px solid alpha(@borders, 0.8);
                                                ",

                                                gtk::Label {
                                                    #[watch]
                                                    set_label: &{
                                                        model.show.as_ref()
                                                            .map(|s| s.title().trim().chars().take(2).collect::<String>().to_uppercase())
                                                            .unwrap_or_default()
                                                    },
                                                    add_css_class: "title-large",
                                                    set_hexpand: true,
                                                    set_vexpand: true,
                                                    set_halign: gtk::Align::Center,
                                                    set_valign: gtk::Align::Center,
                                                    inline_css: "color: @dim_label_opacity; opacity: 0.25; font-weight: 800;",
                                                }
                                            },

                                            add_overlay = &gtk::Picture {
                                                #[watch]
                                                set_paintable: model.show_image_texture.as_ref().map(|t| t.upcast_ref::<adw::gdk::Paintable>()),
                                                #[watch]
                                                set_visible: model.show_image_texture.is_some(),

                                                set_hexpand: true,
                                                set_vexpand: true,
                                                set_halign: gtk::Align::Fill,
                                                set_valign: gtk::Align::Fill,
                                                set_content_fit: gtk::ContentFit::Cover,
                                                set_can_shrink: true,
                                                inline_css: "border-radius: 16px;",
                                            }
                                        },

                                        // Text Detail Layout Column
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_spacing: 6,
                                            set_hexpand: true,
                                            set_valign: gtk::Align::Start,

                                            // Podcast Title Header
                                            gtk::Label {
                                                #[watch]
                                                set_label: model.show.as_ref().map(|s| s.title().trim()).unwrap_or("Loading Show..."),
                                                set_halign: gtk::Align::Start,
                                                set_wrap: true,
                                                set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                                add_css_class: "title-1"
                                            },

                                            // Description Excerpt Block
                                            gtk::Label {
                                                #[watch]
                                                set_use_markup: true,
                                                #[watch]
                                                set_markup: &{
                                                    let raw_markup = if let Some(show) = model.show.as_ref() {
                                                        let desc = show.description();
                                                        let markup = episode_description_parser::html2pango_markup(desc);

                                                        if !markup.is_empty() {
                                                            markup
                                                        } else if !desc.is_empty() {
                                                            html2text::config::plain()
                                                                .string_from_read(desc.as_bytes(), desc.len())
                                                                .unwrap_or_else(|_| desc.to_string())
                                                        } else {
                                                            "".to_string()
                                                        }
                                                    } else {
                                                        "".to_string()
                                                    };

                                                    raw_markup.replace('\n', " ").replace('\r', " ")
                                                },
                                                set_halign: gtk::Align::Start,
                                                set_wrap: true,
                                                set_max_width_chars: 75,
                                                set_lines: 6,
                                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                                set_css_classes: &vec!["dimmed", "body"],
                                            },

                                            gtk::Separator { set_vexpand: true, add_css_class: "spacer" },
                                            gtk::Separator { set_vexpand: true, add_css_class: "spacer" },

                                            // Interactive Row Buttons Layout
                                            gtk::Box {
                                                set_orientation: gtk::Orientation::Horizontal,
                                                set_spacing: 12,
                                                set_halign: gtk::Align::Start,

                                                model.latest_play_button.widget(){
                                                    add_css_class: "suggested-action"
                                                },

                                                gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                                                gtk::Button {
                                                    set_label: "Following",
                                                    add_css_class: "pill",
                                                },

                                                gtk::Button {
                                                    set_icon_name: "view-more-symbolic",
                                                    set_css_classes: &vec!["circular"],
                                                    set_halign: gtk::Align::Center,
                                                    set_valign: gtk::Align::Center,

                                                },
                                            }
                                        }
                                    },

                                    // --- EPISODES HEADER LIST DIVISION ---
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_halign: gtk::Align::Start,
                                        set_spacing: 4,
                                        inline_css: "margin-top: 16px;",

                                        gtk::Label {
                                            set_label: "Episodes",
                                            add_css_class: "title-2",
                                            set_valign: gtk::Align::Center,
                                        },

                                        gtk::Image {
                                            set_icon_name: Some("go-next-symbolic"),
                                            inline_css: "opacity: 0.7; margin-left: 4px;",
                                            set_valign: gtk::Align::Center,
                                        },

                                        add_controller = gtk::GestureClick {
                                            connect_released[sender] => move |_, _, _, _| {
                                                sender.input(ShowPageInput::GotoAllEpisodesPage);
                                            }
                                        }
                                    },

                                    #[local_ref]
                                    episodes_container -> gtk::ListBox {
                                        add_css_class: "boxed-list",
                                    },

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_halign: gtk::Align::Start,
                                        set_valign: gtk::Align::Start,

                                        gtk::Button {
                                            #[watch]
                                            set_label: &format!("See All ({})", model.episode_count),
                                            set_css_classes: &vec!["flat", "accent"],
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Start,

                                            connect_clicked[sender]=> move |_| {
                                                sender.input(ShowPageInput::GotoAllEpisodesPage);
                                            }
                                        }
                                    },

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_halign: gtk::Align::Start,
                                        set_valign: gtk::Align::Start,
                                        set_spacing: 16,

                                        gtk::Label {
                                            set_label: "About",
                                            set_css_classes: &vec!["title-3"],
                                            set_wrap: true,
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Start
                                        },

                                        gtk::Label {
                                            #[watch]
                                            set_use_markup: true,
                                            #[watch]
                                            set_markup: &{
                                                if let Some(desc) = model.show.as_ref().map(|s| s.description()) {
                                                    let markup = episode_description_parser::html2pango_markup(desc);

                                                    // Check if the generated markup is empty or invalid compared to the original input
                                                    if markup.is_empty() && !desc.is_empty() {
                                                        html2text::config::plain()
                                                            .string_from_read(desc.as_bytes(), desc.len())
                                                            .unwrap_or_else(|_| desc.to_string())
                                                    } else {
                                                        markup
                                                    }
                                                } else {
                                                    "".to_string()
                                                }
                                            },
                                            set_css_classes: &vec!["body"],
                                            set_wrap: true,

                                            set_width_chars: 30,
                                            set_max_width_chars: 30,
                                            set_hexpand: false,

                                            set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Start
                                        },

                                        gtk::Label {
                                            set_label: model.show.as_ref().map(|s| s.link()).unwrap_or(""),
                                            set_css_classes: &vec!["accent"],
                                            set_wrap: true,
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Start
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        show_id: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let episodes_parent = gtk::ListBox::builder().build();
        let latest_play_button = PlayButton::builder()
            .launch(PlayButtonInitData {
                label: "Latest Episode".to_string(),
                state: EpisodePlayingState::Stopped,
                progress: 0.0,
            })
            .forward(sender.input_sender(), |msg| match msg {
                PlayButtonOutput::Clicked => ShowPageInput::TogglePlayLatest,
            });

        let all_episodes_page =
            AllEpisodesPage::builder()
                .launch(show_id)
                .forward(sender.output_sender(), |msg| match msg {
                    AllEpisodesPageOutput::TogglePlay(episode_id) => {
                        ShowPageOutput::TogglePlay(episode_id)
                    }
                    AllEpisodesPageOutput::RequestDownload(episode_id) => {
                        ShowPageOutput::RequestDownload(episode_id)
                    }
                    AllEpisodesPageOutput::CancleDownload(episode_id) => {
                        ShowPageOutput::CancleDownload(episode_id)
                    }
                    AllEpisodesPageOutput::SetPlayNext(episode_id) => {
                        ShowPageOutput::SetPlayNext(episode_id)
                    }
                    AllEpisodesPageOutput::AddToPlaylist(episode_id) => {
                        ShowPageOutput::AddToPlaylist(episode_id)
                    }
                    AllEpisodesPageOutput::RequestDeleteEpisode(episode_id) => {
                        ShowPageOutput::RequestDeleteEpisode(episode_id)
                    }
                    AllEpisodesPageOutput::NotifyError(error) => ShowPageOutput::NotifyError(error),
                    AllEpisodesPageOutput::ExecuteAction(uuid, action) => {
                        ShowPageOutput::Execute(uuid, action)
                    }
                });

        let model = Self {
            episodes: FactoryVecDeque::builder().launch(episodes_parent).forward(
                sender.output_sender(),
                |msg| match msg {
                    EpisodeListItemOutput::TogglePlay(id) => ShowPageOutput::TogglePlay(id),
                    EpisodeListItemOutput::RequestDownload(episode_id) => {
                        ShowPageOutput::RequestDownload(episode_id)
                    }
                    EpisodeListItemOutput::CancleDownload(episode_id) => {
                        ShowPageOutput::CancleDownload(episode_id)
                    }
                    EpisodeListItemOutput::NotifyError(error) => ShowPageOutput::NotifyError(error),
                    EpisodeListItemOutput::SetPlayNext(episode_id) => {
                        ShowPageOutput::SetPlayNext(episode_id)
                    }
                    EpisodeListItemOutput::AddToPlaylist(episode_id) => {
                        ShowPageOutput::AddToPlaylist(episode_id)
                    }
                    // EpisodeListItemOutput::GotoEpisode(episode_id) => todo!(),
                    EpisodeListItemOutput::RequestDeleteEpisode(episode_id) => {
                        ShowPageOutput::RequestDeleteEpisode(episode_id)
                    }
                    EpisodeListItemOutput::GotoEpisode(_episode_id) => {
                        ShowPageOutput::NotifyError(format!(""))
                    }
                },
            ),
            index_by_id: HashMap::new(),
            episode_count: 0,
            latest_play_button,
            show: None,
            show_image_texture: None,
            latest_episode: None,
            all_episodes_page,
            task_id: Uuid::new_v4(),
        };

        let episodes_container = model.episodes.widget();

        let widgets = view_output!();

        let _ = sender.output(ShowPageOutput::Execute(
            model.task_id,
            Action::FetchShowEpisodes(show_id),
        ));
        sender.input(ShowPageInput::GetShow(show_id));
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ShowPageInput::GetShow(show_id) => {
                let show_result = dbqueries::get_podcast_from_id(show_id);
                sender.input(ShowPageInput::ShowGotten(show_result));
            }
            ShowPageInput::ShowGotten(show_result) => match show_result {
                Ok(show) => {
                    self.show = Some(show);
                    if let Some(show) = &self.show {
                        if let Some(image_url_ref) = show.image_uri() {
                            let image_url = image_url_ref.to_string();
                            sender.oneshot_command(async move {
                                let downloaded_texture =
                                    fetch_cached_image(&image_url, ImageSize::from_dimesion(500))
                                        .await;

                                ShowPageCmdInput::DownloadImage(downloaded_texture)
                            });
                        }
                    }
                }
                Err(error) => {
                    let _ = sender.output(ShowPageOutput::NotifyError(format!(
                        "Failed to load show: {error}"
                    )));
                }
            },
            ShowPageInput::ImageDownloaded(opt_texture) => {
                self.show_image_texture = opt_texture;
            }
            ShowPageInput::DownloadStarted(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes
                        .send(index.current_index(), EpisodeListItemInput::DownloadStarted);
                }
                self.all_episodes_page
                    .emit(AllEpisodesPageInput::DownloadStarted(episode_id));
            }
            ShowPageInput::DownloadProgress(episode_id, fraction) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::DownloadProgress(fraction),
                    );
                }
                self.all_episodes_page
                    .emit(AllEpisodesPageInput::DownloadProgress(episode_id, fraction));
            }
            ShowPageInput::DownloadCancled(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes
                        .send(index.current_index(), EpisodeListItemInput::DownloadCancled);
                }
                self.all_episodes_page
                    .emit(AllEpisodesPageInput::DownloadCancled(episode_id));
            }
            ShowPageInput::DownloadFinished(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::DownloadFinished,
                    );
                }
                self.all_episodes_page
                    .emit(AllEpisodesPageInput::DownloadFinished(episode_id));
            }
            ShowPageInput::ChangePlayBackState(state, episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::ChangePlayBackState(state),
                    );
                }

                self.all_episodes_page
                    .emit(AllEpisodesPageInput::ChangePlayBackState(state, episode_id));

                if let Some(episode) = self.latest_episode {
                    if episode == episode_id {
                        match state {
                            PlayState::Stopped => {
                                self.latest_play_button
                                    .emit(PlayButtonInput::UpdatePlayingState(
                                        EpisodePlayingState::Stopped,
                                    ));

                                self.latest_play_button
                                    .emit(PlayButtonInput::SetLabel("Latest Episode".to_string()));
                            }
                            PlayState::Buffering => {
                                self.latest_play_button
                                    .emit(PlayButtonInput::UpdatePlayingState(
                                        EpisodePlayingState::Playing,
                                    ));
                            }
                            PlayState::Paused => {
                                self.latest_play_button
                                    .emit(PlayButtonInput::UpdatePlayingState(
                                        EpisodePlayingState::Paused,
                                    ));
                            }
                            PlayState::Playing => {
                                self.latest_play_button
                                    .emit(PlayButtonInput::UpdatePlayingState(
                                        EpisodePlayingState::Playing,
                                    ));
                            }
                            _ => {}
                        }
                    }
                }
            }
            ShowPageInput::PlayBackProgress(episode_id, pos, rem) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::PlayBackProgress(pos, rem),
                    );
                }

                self.all_episodes_page
                    .emit(AllEpisodesPageInput::PlayBackProgress(episode_id, pos, rem));

                if let Some(episode) = self.latest_episode {
                    if episode == episode_id {
                        self.latest_play_button
                            .emit(PlayButtonInput::UpdateProgress(pos));

                        let duration_str = if rem > 0 {
                            let hours = rem / 3600;
                            let minutes = (rem % 3600) / 60;
                            let seconds = rem % 60;

                            if hours > 0 {
                                format!("{}h {}m", hours, minutes)
                            } else if minutes > 0 {
                                format!("{}m", minutes)
                            } else {
                                format!("{}s", seconds)
                            }
                        } else {
                            "0s".to_string()
                        };

                        self.latest_play_button
                            .emit(PlayButtonInput::SetLabel(duration_str));
                    }
                }
            }
            ShowPageInput::TogglePlayLatest => {
                if let Some(episode) = self.latest_episode {
                    let _ = sender.output(ShowPageOutput::TogglePlay(episode));
                }
            }
            ShowPageInput::ChangeEpisodeTo(episode_id) => {
                if let Some(episode) = self.latest_episode {
                    if episode != episode_id {
                        sender.input(ShowPageInput::ChangePlayBackState(
                            PlayState::Stopped,
                            episode,
                        ));
                    }
                }

                self.all_episodes_page
                    .emit(AllEpisodesPageInput::ChangeEpisodeTo(episode_id));

                self.episodes
                    .broadcast(EpisodeListItemInput::ChangeEpisodeTo(episode_id));
            }
            ShowPageInput::GotoAllEpisodesPage => {
                widgets.nav_view.push(self.all_episodes_page.widget());
            }
            ShowPageInput::EpisodeDeleted(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes
                        .send(index.current_index(), EpisodeListItemInput::EpisodeDeleted);
                }
                self.all_episodes_page
                    .emit(AllEpisodesPageInput::EpisodeDeleted(episode_id));
            }
            ShowPageInput::ActionFinished(uuid, result) => {
                if uuid == self.task_id {
                    if let Ok(episodes_arc) = result.downcast::<Vec<Episode>>() {
                        if let Ok(episodes) = Arc::try_unwrap(episodes_arc) {
                            sender.input(ShowPageInput::LoadEpisodes(episodes));
                        }
                    }
                }
            }
            ShowPageInput::LoadEpisodes(episodes) => {
                if let Some(episode) = episodes.first() {
                    self.latest_episode = Some(episode.clone().id());
                }
                let mut guard = self.episodes.guard();
                guard.clear();

                for episode in episodes.iter().take(10) {
                    let index = guard.push_back(episode.clone());

                    self.index_by_id.insert(episode.id(), index);
                }
                self.episode_count = episodes.len();
            }
        }

        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ShowPageCmdInput::DownloadImage(opt_texture) => {
                sender.input(ShowPageInput::ImageDownloaded(opt_texture));
            }
        }
    }
}
