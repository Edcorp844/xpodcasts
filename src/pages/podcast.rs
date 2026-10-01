use std::collections::HashMap;

use adw::gdk::Texture;
use adw::prelude::*;
use gst_play::PlayState;
use podcasts_data::{
    Episode, EpisodeId, FEED_MANAGER, Show, Source, dbqueries, discovery::FoundPodcast,
};
use relm4::{Component, prelude::*};
use uuid::Uuid;

use crate::{
    components::{
        episode_list_item::{EpisodeListItem, EpisodeListItemInput, EpisodeListItemOutput},
        play_button::{
            EpisodePlayingState, PlayButton, PlayButtonInitData, PlayButtonInput, PlayButtonOutput,
        },
    }, util::{
        cover_image::{ImageSize, fetch_cached_image},
        episode_description_parser,
    }, workers::action_worker::worker::ActionResult,
};

#[derive(Debug)]
pub struct PodcastPage {
    podcast: FoundPodcast,
    cover_texture: Option<Texture>,
    episodes: FactoryVecDeque<EpisodeListItem>,
    latest_play_button: Controller<PlayButton>,
    loading_episodes: bool,
    latest_episode: Option<EpisodeId>,
    show: Option<Show>,
    episode_count: usize,
    subscribed: Option<bool>,
    index_by_id: HashMap<EpisodeId, relm4::factory::DynamicIndex>,
}

#[derive(Debug)]
pub enum PodcastPageInput {
    ActionFinished(Uuid, ActionResult),
    ImageDownloaded(Option<Texture>),
    SetLoadingEpisodes(bool),
    GetEpisodes,
    Show(Show),
    Episoded(Vec<Episode>),
    StreamLatestEpisode,
    Subscribe,
    SetSubscriptionStatus(bool),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
}

#[derive(Debug)]
pub enum PodcastPageOutput {
    TogglePlay(EpisodeId),
    NotifyError(String),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    Subscribe(String),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    GotoEpisode(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
}

#[derive(Debug)]
pub enum PodcastPageCmdInput {
    DownloadImage(Option<Texture>),
}

#[relm4::component(pub)]
impl Component for PodcastPage {
    type Init = FoundPodcast;
    type Input = PodcastPageInput;
    type Output = PodcastPageOutput;
    type CommandOutput = PodcastPageCmdInput;

    fn init(
        podcast: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let latest_play_button = PlayButton::builder()
            .launch(PlayButtonInitData {
                label: "Latest Episode".to_string(),
                state: EpisodePlayingState::Stopped,
                progress: 0.0,
            })
            .forward(sender.input_sender(), |msg| match msg {
                PlayButtonOutput::Clicked => PodcastPageInput::StreamLatestEpisode,
            });

        let episodes_parent = gtk::ListBox::builder().build();
        let model = PodcastPage {
            episodes: FactoryVecDeque::builder().launch(episodes_parent).forward(
                sender.output_sender(),
                |msg| match msg {
                    EpisodeListItemOutput::TogglePlay(id) => PodcastPageOutput::TogglePlay(id),
                    EpisodeListItemOutput::RequestDownload(episode_id) => {
                        PodcastPageOutput::RequestDownload(episode_id)
                    }
                    EpisodeListItemOutput::CancleDownload(episode_id) => {
                        PodcastPageOutput::CancleDownload(episode_id)
                    }
                    EpisodeListItemOutput::NotifyError(error) => {
                        PodcastPageOutput::NotifyError(error)
                    }
                    EpisodeListItemOutput::GotoEpisode(episode_id) => {
                        PodcastPageOutput::GotoEpisode(episode_id)
                    }
                    EpisodeListItemOutput::SetPlayNext(episode_id) => {
                        PodcastPageOutput::SetPlayNext(episode_id)
                    }
                    EpisodeListItemOutput::AddToPlaylist(episode_id) => {
                        PodcastPageOutput::AddToPlaylist(episode_id)
                    }
                    EpisodeListItemOutput::RequestDeleteEpisode(episode_id) => {
                        PodcastPageOutput::RequestDeleteEpisode(episode_id)
                    }
                },
            ),
            podcast,
            cover_texture: None,
            loading_episodes: true,
            show: None,
            episode_count: 0,
            latest_episode: None,
            latest_play_button,
            subscribed: None,
            index_by_id: HashMap::new(),
        };

        let episodes_container = model.episodes.widget();

        let widgets = view_output!();

        let image_url = model.podcast.art.clone();
        sender.oneshot_command(async move {
            let downloaded_texture =
                fetch_cached_image(&image_url, ImageSize::from_dimesion(500)).await;

            PodcastPageCmdInput::DownloadImage(downloaded_texture)
        });

        sender.input(PodcastPageInput::GetEpisodes);
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
            PodcastPageInput::GetEpisodes => {
                let feed = self.podcast.feed.clone();
                let sender_clone = sender.clone();
                relm4::tokio::spawn(async move {
                    Self::get_episodes(sender_clone, feed.clone()).await;
                });
            }
            PodcastPageInput::SetSubscriptionStatus(status) => {
                self.subscribed = Some(status);
            }
            PodcastPageInput::SetLoadingEpisodes(state) => {
                println!("loadind episodes: {state}");
            }
            PodcastPageInput::Show(show) => {
                self.show = Some(show);
            }
            PodcastPageInput::ImageDownloaded(texture) => {
                self.cover_texture = texture;
            }
            PodcastPageInput::Episoded(episodes) => {
                let mut guard = self.episodes.guard();
                guard.clear();

                if let Some(episode) = episodes.first() {
                    self.latest_episode = Some(episode.id());
                }

                for episode in episodes.iter().take(10) {
                    let index = guard.push_back(episode.clone());

                    self.index_by_id.insert(episode.id(), index);
                }
                self.episode_count = episodes.len();
            }
            PodcastPageInput::Subscribe => {
                let _ = sender.output(PodcastPageOutput::Subscribe(self.podcast.feed.clone()));
            }
            PodcastPageInput::StreamLatestEpisode => {
                if let Some(episode) = self.latest_episode {
                    let _ = sender.output(PodcastPageOutput::TogglePlay(episode));
                }
            }
            PodcastPageInput::DownloadStarted(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes
                        .send(index.current_index(), EpisodeListItemInput::DownloadStarted);
                }
            }
            PodcastPageInput::DownloadCancled(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes
                        .send(index.current_index(), EpisodeListItemInput::DownloadCancled);
                }
            }
            PodcastPageInput::DownloadProgress(episode_id, fraction) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::DownloadProgress(fraction),
                    );
                }
            }
            PodcastPageInput::DownloadFinished(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::DownloadFinished,
                    );
                }
            }
            PodcastPageInput::ChangePlayBackState(state, episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::ChangePlayBackState(state),
                    );
                }

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

            PodcastPageInput::PlayBackProgress(episode_id, pos, rem) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::PlayBackProgress(pos, rem),
                    );
                }

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
            PodcastPageInput::ChangeEpisodeTo(episode_id) => {
                if let Some(episode) = self.latest_episode {
                    if episode != episode_id {
                        sender.input(PodcastPageInput::ChangePlayBackState(
                            PlayState::Stopped,
                            episode,
                        ));
                    }
                }

                self.episodes
                    .broadcast(EpisodeListItemInput::ChangeEpisodeTo(episode_id));
            }
            PodcastPageInput::ActionFinished(uuid, any) => {
                
            },
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
            PodcastPageCmdInput::DownloadImage(opt_texture) => {
                sender.input(PodcastPageInput::ImageDownloaded(opt_texture));
            }
        }
    }

    view! {
        adw::NavigationPage {
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
                            set_margin_bottom:40,

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
                                                model.podcast.title.trim().chars().take(2).collect::<String>().to_uppercase()

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
                                        set_paintable: model.cover_texture.as_ref().map(|t| t.upcast_ref::<adw::gdk::Paintable>()),
                                        #[watch]
                                        set_visible: model.cover_texture.is_some(),

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
                                        set_label: model.podcast.title.trim(),
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
                                            let desc = &model.podcast.description;
                                            let markup = episode_description_parser::html2pango_markup(&desc);

                                            markup
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

                                        gtk::Box{
                                            #[watch]
                                            set_visible: model.latest_episode.is_some(),
                                            model.latest_play_button.widget(),
                                        },

                                        gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                                        gtk::Button {
                                            #[watch]
                                            set_label: {
                                                match model.subscribed {
                                                    Some(true) => "Following",
                                                    Some(false) => "+ Follow",
                                                    None => "Loading...",
                                                }
                                            },
                                            set_css_classes: &vec!["pill", "suggested-action"],
                                            #[watch]
                                            set_visible: model.subscribed.is_some(),

                                            connect_clicked[sender]=>move|_|{
                                                sender.input(PodcastPageInput::Subscribe)
                                            }
                                        },

                                        gtk::Button {
                                            set_icon_name: "view-more-symbolic",
                                            set_css_classes: &vec![ "circular"],
                                            set_halign: gtk::Align::Center,
                                            set_valign: gtk::Align::Center,
                                        },
                                    }
                                }
                            },

                            // --- EPISODES HEADER LIST DIVISION ---
                            gtk::Box {
                                #[watch]
                                set_visible: model.episode_count> 0,
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
                                }
                            },

                            #[local_ref]
                            episodes_container -> gtk::ListBox {
                                #[watch]
                                set_visible: model.episode_count > 0,
                                add_css_class: "boxed-list",
                            },

                            gtk::Box{
                                set_halign: gtk::Align::Fill,
                                set_valign: gtk::Align::Fill,
                                #[watch]
                                set_visible: model.loading_episodes,

                                adw::Spinner{
                                    set_halign: gtk::Align::Center,
                                    set_valign: gtk::Align::Center,
                                }
                            },

                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_halign: gtk::Align::Start,
                                set_valign: gtk::Align::Start,

                                gtk::Button {
                                    #[watch]
                                    set_visible: model.episode_count > 10,
                                    #[watch]
                                    set_label: &format!("See All ({})", model.episode_count),
                                    set_css_classes: &vec!["flat", "accent"],
                                    set_halign: gtk::Align::Start,
                                    set_valign: gtk::Align::Start
                                }
                            },

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_halign: gtk::Align::Start,
                                set_valign: gtk::Align::Start,
                                set_spacing: 16,

                                gtk::Label {
                                    #[watch]
                                    set_visible: model.show.is_some(),
                                    set_label: "About",
                                    set_css_classes: &vec!["title-3"],
                                    set_wrap: true,
                                    set_halign: gtk::Align::Start,
                                    set_valign: gtk::Align::Start
                                },

                                gtk::Label {
                                    #[watch]
                                    set_visible: model.show.is_some(),
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

                                    // Constraints to snap the layout to a tight, narrow column
                                    set_width_chars: 30,
                                    set_max_width_chars: 30,
                                    set_hexpand: false,

                                    set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                    set_halign: gtk::Align::Start,
                                    set_valign: gtk::Align::Start
                                },

                                 gtk::Label {
                                    set_label: "Information",
                                    set_css_classes: &vec!["title-3"],
                                    set_wrap: true,
                                    set_halign: gtk::Align::Start,
                                    set_valign: gtk::Align::Start
                                },

                                gtk::Box{
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_halign: gtk::Align::Fill,
                                    set_valign: gtk::Align::Start,
                                    set_spacing: 24,

                                    gtk::Box{
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_spacing: 16,

                                        gtk::Label {
                                            set_label: "Author",
                                            set_css_classes: &vec!["dimmed", "heading"],
                                            set_wrap: true,
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Start
                                        },

                                        gtk::Label{
                                            set_label:  &model.podcast.author,
                                            add_css_class: "body",
                                            set_halign: gtk::Align::Start,
                                            set_wrap: true
                                        },
                                    },

                                    gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                                    gtk::Box{
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_spacing: 16,

                                        gtk::Label {
                                            set_label: "Last Updated",
                                            set_css_classes: &vec!["dimmed", "heading"],
                                            set_wrap: true,
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Start
                                        },

                                        gtk::Label{
                                            set_label:   &match &model.podcast.last_publication {
                                                Some(date) => {
                                                    format!("{}", date.format("%-d %B %Y"))
                                                }
                                                None => "".to_string(),
                                            },

                                            add_css_class: "body",
                                            set_halign: gtk::Align::Start,
                                            set_wrap: true
                                        },
                                    },

                                    gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                                    gtk::Box{
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_spacing: 16,

                                        gtk::Label {
                                            set_label: "Episodes",
                                            set_css_classes: &vec!["dimmed", "heading"],
                                            set_wrap: true,
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Start
                                        },

                                        gtk::Label{
                                            set_label:   &match &model.podcast.episode_count {
                                                Some(episodes) => {
                                                    format!("{}", episodes)
                                                }
                                                None => "Unknown".to_string(),
                                            },

                                            add_css_class: "body",
                                            set_halign: gtk::Align::Start,
                                            set_wrap: true
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
}

impl PodcastPage {
    pub async fn get_episodes(sender: ComponentSender<Self>, feed: String) {
        let mut temporary_cleanup_needed = false;

        if let Ok(source) = dbqueries::get_source_from_uri(&feed) {
            if let Ok(show) = dbqueries::get_podcast_from_source_id(source.id()) {
                sender.input(PodcastPageInput::Show(show.clone()));
                sender.input(PodcastPageInput::SetSubscriptionStatus(true));

                // If we have local cached episodes, push them immediately so the screen isn't blank
                if let Ok(episodes) = dbqueries::get_pd_episodes(&show) {
                    sender.input(PodcastPageInput::Episoded(episodes));
                }
            }
        }

        // --- STAGE 2: Heavy Network and Database Work ---
        if let Err(e) = async {
            let source = match dbqueries::get_source_from_uri(&feed) {
                Ok(src) => src,
                Err(_) => {
                    let src = Source::from_url(&feed)?;
                    temporary_cleanup_needed = true;
                    src
                }
            };

            let source_id = source.id();
            sender.input(PodcastPageInput::SetLoadingEpisodes(true));

            let _ = FEED_MANAGER.refresh(vec![source]).await;

            let show = dbqueries::get_podcast_from_source_id(source_id)?;
            let episodes = dbqueries::get_pd_episodes(&show)?;

            sender.input(PodcastPageInput::Show(show));
            sender.input(PodcastPageInput::Episoded(episodes));
            sender.input(PodcastPageInput::SetSubscriptionStatus(
                !temporary_cleanup_needed,
            ));
            sender.input(PodcastPageInput::SetLoadingEpisodes(false));

            Ok::<(), anyhow::Error>(())
        }
        .await
        {
            println!("error: {}", e);
            let _ = sender.output(PodcastPageOutput::NotifyError(e.to_string()));
            sender.input(PodcastPageInput::SetLoadingEpisodes(false));
        }
    }
}
