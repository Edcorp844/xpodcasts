use gettextrs::gettext;
use gst_play::PlayState;
use podcasts_data::EpisodeId;

use adw::prelude::WidgetExt;
use std::collections::HashMap;
use uuid::Uuid;

use crate::app_navigation_ext::{NavigationPage, PageController};
use adw;
use relm4::{adw::prelude::*, prelude::*, Component, ComponentSender, Controller};

use crate::components::{
    main_menu_button::MainMenuButton,
    miniplayer::{MiniPlayerModel, MiniplayerModelInput, MiniplayerModelOutput, PlayerPageView},
};

use crate::pages::{
    downloads::{DownloadsPage, DownloadsPageOutput},
    home::{HomPageOutput, HomePage},
    new::NewPage,
    player_page::{PlayerPage, PlayerPageInput, PlayerPageOutput},
    recents::{RecentlyUpdatedPage, RecentlyUpdatedPageOutput},
    search::{SearchPage, SearchPageOutput},
    shows::{ShowsPage, ShowsPageInput, ShowsPageOutput},
};

use crate::workers::{
    action_worker::{
        action::{Action, SearchActionResultCategory},
        worker::{ActionResult, ActionWorker, ActionWorkerInput, ActionWorkerOutput},
    },
    database_worker::worker::{DatabaseWoker, DatabaseWokerInput, DatabaseWokerOutput},
    gstremer_worker::worker::{GStreamerWorker, GStreamerWorkerInput, GStreamerWorkerOutput},
};

pub struct AppModel {
    is_sidebar_visible: bool,
    is_search_page_visible: bool,
    search_page: PageController,
    main_menu_button: Controller<MainMenuButton>,
    player_page: Controller<PlayerPage>,
    pages_cache: HashMap<String, PageController>,
    current_page_key: String,
    miniplayer: Controller<MiniPlayerModel>,
    worker_controller: Controller<ActionWorker>,
    player: Controller<GStreamerWorker>,
    database: Controller<DatabaseWoker>,
    is_loading: bool,
    compact_layout: bool,
    is_narrow: bool,
}

#[derive(Debug)]
pub enum AppModelInput {
    ExecuteAction(Uuid, Action),
    ActionFinished(Uuid, ActionResult),
    ToggleSidebar,
    StartLoading,
    StopLoading,
    ToggleShowSearchPage(bool),
    TogglePlayBack,
    SelectPage(NavigationPage),
    SetSidebarCollapsed(bool),
    RefreshShowsPage,
    SeekAudioPosition(f64),
    TogglePlay(EpisodeId),
    NotifyError(String),
    Subscribe(String),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    SetCurrentEpisode(EpisodeId),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
    EpisodeDeleted(EpisodeId),
    SetVolume(f64),
    VolumeValue(f64),
    ShowPlayerPage(PlayerPageView),
    AddToPlaylist(EpisodeId),
    UpdatePlaylist(Vec<EpisodeId>, Option<usize>),
    SetPlayNext(EpisodeId),
    SetPlayNow(EpisodeId),
    RemoveFromPlayList(EpisodeId),
    RequestMute,
    RequestUnmute,
    RequestVolumeValue,
    Seekforward,
    SeekBakward,
    RequestSearch(Uuid, String, SearchActionResultCategory),
    NavigationBreakpointChanged(bool),
    None,
}

#[derive(Debug)]
pub enum AppModelOutput {}

#[relm4::component(pub)]
impl Component for AppModel {
    type Init = ();
    type Input = AppModelInput;
    type Output = AppModelOutput;
    type CommandOutput = ();

    view! {
            adw::ApplicationWindow {
                set_default_size: (1080 ,800),
                #[wrap(Some)]
                #[name = "nav_view"]
                set_content = &adw::NavigationView {
                    add=&adw::NavigationPage {
                        set_title: "XPodcats",
                        #[wrap(Some)]
                        #[name="split_view"]
                        set_child=&adw::OverlaySplitView {
                        #[watch]
                        set_show_sidebar: !model.compact_layout,

                        set_pin_sidebar: true,
                        set_sidebar_position: gtk::PackType::Start,

                        connect_collapsed_notify => move |view| {
                            AppModelInput::SetSidebarCollapsed(view.is_collapsed());
                        },

                        #[wrap(Some)]
                        set_sidebar =  &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                #[name(search_button)]
                                pack_start = &gtk::ToggleButton {
                                    set_icon_name: "edit-find-symbolic",
                                    set_tooltip: "Search",
                                    add_css_class: "flat",

                                    connect_clicked[sender] => move |btn| {
                                            sender.input(AppModelInput::ToggleShowSearchPage(btn.is_active()));
                                    }
                                },
                                pack_end = model.main_menu_button.widget(),
                            },

                            #[wrap(Some)]
                            set_content = &adw::Sidebar{


                                append = adw::SidebarSection {
                                    append= adw::SidebarItem{
                                        set_icon_name: Some(&NavigationPage::Home.icon_name()),
                                        set_title: Some(&NavigationPage::Home.title()),
                                    },
                                     append= adw::SidebarItem{
                                       set_icon_name: Some(&NavigationPage::New.icon_name()),
                                        set_title: Some(&NavigationPage::New.title()),
                                    }
                                },
                                append = adw::SidebarSection {
                                    set_title: Some(&gettext("Library")),
                                    append= adw::SidebarItem{
                                        set_icon_name: Some(&NavigationPage::Recents.icon_name()),
                                        set_title: Some(&NavigationPage::Recents.title()),
                                    },
                                     append= adw::SidebarItem{
                                        set_icon_name: Some(&NavigationPage::Shows.icon_name()),
                                        set_title: Some(&NavigationPage::Shows.title()),
                                    },
                                     append= adw::SidebarItem{
                                       set_icon_name: Some(&NavigationPage::Downloads.icon_name()),
                                        set_title: Some(&NavigationPage::Downloads.title()),
                                    }
                                },

                                connect_activated[sender] => move |bar, _index| {
                                    if let Some(item) = bar.selected_item() {
                                        if let Some(title) = item.title() {
                                            let title = title.as_str();
                                            sender.input(AppModelInput::SelectPage(NavigationPage::from_name(title)));
                                        }
                                    }
                                }

                            },
                        },

                        #[wrap(Some)]
                        #[name = "toast_overlay"]
                        set_content = &adw::ToastOverlay {
                            adw::NavigationPage {
                                set_tag: Some("main-content"),
                                set_title: "Podcasts",

                                #[wrap(Some)]
                                set_child = &adw::ToolbarView {
                                    add_top_bar = &gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        model.miniplayer.widget(),
                                        gtk::Separator {
                                            inline_css: "min-height: 2px; border: none; background: linear-gradient(90deg, rgba(0, 122, 255, 0) 0%, #007AFF 25%, #AF52DE 50%, #FF2D55 75%, rgba(255, 45, 85, 0) 100% ); background-size: 200% 100%; animation: shimmer-flow 1.5s infinite linear;",
                                            #[watch]
                                            set_visible: model.is_loading,
                                            set_halign: gtk::Align::Fill,
                                        },
                                    },



                                 #[wrap(Some)]
    #[name = "content_bin"]
    set_content = &adw::Bin {
        #[wrap(Some)]
        #[name = "mobile_stack"]
        set_child = &adw::ViewStack {},
    },

    #[name = "mobile_switcher_bar"]
    add_bottom_bar = &adw::ViewSwitcherBar {
        #[watch]
        set_visible: model.compact_layout,
        set_reveal: true,
        set_stack: Some(&mobile_stack),
    },

                                },
                            }
                        },
                    },
                },
                }
            }
        }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let worker_controller =
            ActionWorker::builder()
                .launch(())
                .forward(sender.input_sender(), move |message| match message {
                    ActionWorkerOutput::NotifyError(error) => AppModelInput::NotifyError(error),
                    ActionWorkerOutput::RefreshAllPages => AppModelInput::None,
                    ActionWorkerOutput::ActionStarted(_id) => AppModelInput::StartLoading,
                    ActionWorkerOutput::ActionsCompleted => AppModelInput::StopLoading,
                    ActionWorkerOutput::ActionCompleted(id, result) => {
                        AppModelInput::ActionFinished(id, result)
                    }
                });

        let main_menu_button = MainMenuButton::builder().launch(()).detach();

        let homepage =
            HomePage::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    HomPageOutput::Subscribe(feed) => AppModelInput::Subscribe(feed),
                    HomPageOutput::ToggleSideBar => AppModelInput::ToggleSidebar,
                    HomPageOutput::TogglePlay(episode_id) => AppModelInput::TogglePlay(episode_id),
                    HomPageOutput::NotifyError(error) => AppModelInput::NotifyError(error),
                    HomPageOutput::RequestDownload(episode_id) => {
                        AppModelInput::RequestDownload(episode_id)
                    }
                    HomPageOutput::CancleDownload(episode_id) => {
                        AppModelInput::CancleDownload(episode_id)
                    }

                    HomPageOutput::SetPlayNext(episode_id) => {
                        AppModelInput::SetPlayNext(episode_id)
                    }
                    HomPageOutput::AddToPlaylist(episode_id) => {
                        AppModelInput::AddToPlaylist(episode_id)
                    }
                    HomPageOutput::RequestDeleteEpisode(episode_id) => {
                        AppModelInput::RequestDeleteEpisode(episode_id)
                    }
                    HomPageOutput::Execute(uuid, action) => {
                        AppModelInput::ExecuteAction(uuid, action)
                    }
                });

        let mut initial_cache = HashMap::new();
        let key = NavigationPage::Home.to_key();

        initial_cache.insert(key.clone(), PageController::Home(homepage));

        let search_page = SearchPage::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                SearchPageOutput::Subscribe(feed) => AppModelInput::Subscribe(feed),
                SearchPageOutput::UpdateISSearching(_state) => AppModelInput::None,
                SearchPageOutput::TogglePlay(episode) => AppModelInput::TogglePlay(episode),
                SearchPageOutput::NotifyError(error) => AppModelInput::NotifyError(error),
                SearchPageOutput::RequestDownload(episode_id) => {
                    AppModelInput::RequestDownload(episode_id)
                }
                SearchPageOutput::CancleDownload(episode_id) => {
                    AppModelInput::CancleDownload(episode_id)
                }
                SearchPageOutput::SetPlayNext(episode_id) => AppModelInput::SetPlayNext(episode_id),
                SearchPageOutput::AddToPlaylist(episode_id) => {
                    AppModelInput::AddToPlaylist(episode_id)
                }
                SearchPageOutput::RequestDeleteEpisode(episode_id) => {
                    AppModelInput::RequestDeleteEpisode(episode_id)
                }
                SearchPageOutput::RequestSearch {
                    task_id,
                    search_text,
                    search_category,
                } => AppModelInput::RequestSearch(
                    task_id,
                    search_text.clone(),
                    search_category.clone(),
                ),
            });

        let miniplayer =
            MiniPlayerModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    MiniplayerModelOutput::TogglePlay => AppModelInput::TogglePlayBack,
                    MiniplayerModelOutput::NotifyError(error) => AppModelInput::NotifyError(error),
                    MiniplayerModelOutput::SeekAudioPosition(pos_fraction) => {
                        AppModelInput::SeekAudioPosition(pos_fraction)
                    }
                    MiniplayerModelOutput::SetVolume(fraction) => {
                        AppModelInput::SetVolume(fraction)
                    }
                    MiniplayerModelOutput::RequestMute => AppModelInput::RequestMute,
                    MiniplayerModelOutput::RequestUnmute => AppModelInput::RequestUnmute,
                    MiniplayerModelOutput::RequestVolumeValue => AppModelInput::RequestVolumeValue,
                    MiniplayerModelOutput::Seekforward => AppModelInput::Seekforward,
                    MiniplayerModelOutput::SeekBakward => AppModelInput::SeekBakward,
                    MiniplayerModelOutput::ShowPlayerPage(player_page_view) => {
                        AppModelInput::ShowPlayerPage(player_page_view)
                    }
                });

        let player_page = PlayerPage::builder().launch(()).forward(
            sender.input_sender(),
            |message| match message {
                PlayerPageOutput::NotifyError(error) => AppModelInput::NotifyError(error),
                PlayerPageOutput::TogglePlay => AppModelInput::TogglePlayBack,
                PlayerPageOutput::SeekAudioPosition(pos) => AppModelInput::SeekAudioPosition(pos),
                PlayerPageOutput::Seekforward => AppModelInput::Seekforward,
                PlayerPageOutput::SeekBakward => AppModelInput::SeekBakward,
                PlayerPageOutput::SetPlayNext(episode_id) => AppModelInput::SetPlayNext(episode_id),
                PlayerPageOutput::SetPlayNow(episode_id) => AppModelInput::SetPlayNow(episode_id),
                PlayerPageOutput::RemoveFromPlayList(episode_id) => {
                    AppModelInput::RemoveFromPlayList(episode_id)
                }
            },
        );

        let player =
            GStreamerWorker::builder()
                .launch(())
                .forward(sender.input_sender(), |message| match message {
                    GStreamerWorkerOutput::PlayBackProgress(episode_id, pos, rem) => {
                        AppModelInput::PlayBackProgress(episode_id, pos, rem)
                    }
                    GStreamerWorkerOutput::NotifyError(error) => AppModelInput::NotifyError(error),
                    GStreamerWorkerOutput::StateChanged(play_state, episode_id) => {
                        AppModelInput::ChangePlayBackState(play_state, episode_id)
                    }
                    GStreamerWorkerOutput::SetCurrentEpisode(episode_id) => {
                        AppModelInput::SetCurrentEpisode(episode_id)
                    }
                    GStreamerWorkerOutput::VolumeValue(vol) => AppModelInput::VolumeValue(vol),
                    GStreamerWorkerOutput::UpdatePlaylist(episode_ids, current) => {
                        AppModelInput::UpdatePlaylist(episode_ids, current)
                    }
                    GStreamerWorkerOutput::Muted => todo!(),
                    GStreamerWorkerOutput::UnMuted => todo!(),
                });

        let database =
            DatabaseWoker::builder()
                .launch(())
                .forward(sender.input_sender(), |message| match message {
                    DatabaseWokerOutput::NotifyError(error) => AppModelInput::NotifyError(error),
                    DatabaseWokerOutput::DownloadStarted(episode_id) => {
                        AppModelInput::DownloadStarted(episode_id)
                    }
                    DatabaseWokerOutput::DownloadProgress {
                        episode_id,
                        fraction,
                    } => AppModelInput::DownloadProgress(episode_id, fraction),
                    DatabaseWokerOutput::DownloadFinished(episode_id) => {
                        AppModelInput::DownloadFinished(episode_id)
                    }
                    DatabaseWokerOutput::DownloadCancelled(episode_id) => {
                        AppModelInput::DownloadCancled(episode_id)
                    }
                    DatabaseWokerOutput::EpisodeDeleted(episode_id) => {
                        AppModelInput::EpisodeDeleted(episode_id)
                    }
                });

        let model = AppModel {
            is_sidebar_visible: true,
            is_search_page_visible: false,
            search_page: PageController::Search(search_page),
            main_menu_button,
            pages_cache: initial_cache,
            current_page_key: key,
            player_page,
            miniplayer,
            worker_controller,
            is_loading: false,
            player,
            database,
            compact_layout: false,
            is_narrow: false,
        };
        // Generates the correct modern auto-derived struct layout type
        let widgets = view_output!();

        let narrow_bp =
            adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 760px").unwrap());
        root.add_breakpoint(narrow_bp);

        let s = sender.clone();
        root.connect_current_breakpoint_notify(move |window| {
            s.input(AppModelInput::NavigationBreakpointChanged(
                window.current_breakpoint().is_some(),
            ));
        });

        widgets
            .mobile_switcher_bar
            .set_stack(Some(&widgets.mobile_stack));
        widgets.mobile_switcher_bar.set_reveal(true);

        // Home (real, already cached)
        if let Some(home) = model.pages_cache.get(&NavigationPage::Home.to_key()) {
            widgets.mobile_stack.add_titled_with_icon(
                home.widget(),
                Some(&NavigationPage::Home.to_key()),
                &NavigationPage::Home.title(),
                &NavigationPage::Home.icon_name(),
            );
        }

        // New (placeholder — built on first SelectPage)
        widgets.mobile_stack.add_titled_with_icon(
            &adw::Bin::new(),
            Some(&NavigationPage::New.to_key()),
            &NavigationPage::New.title(),
            &NavigationPage::New.icon_name(),
        );

        // Library (a NavigationView wrapping the library list)
        {
            let library_nav = adw::NavigationView::new();
            let list = gtk::ListBox::builder()
                .css_classes(["boxed-list"])
                .selection_mode(gtk::SelectionMode::None)
                .margin_top(24)
                .margin_bottom(24)
                .margin_start(24)
                .margin_end(24)
                .build();
            for page in [
                NavigationPage::Recents,
                NavigationPage::Shows,
                NavigationPage::Downloads,
            ] {
                let row = adw::ActionRow::builder()
                    .title(page.title())
                    .activatable(true)
                    .build();
                row.add_prefix(&gtk::Image::from_icon_name(&page.icon_name()));
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

                let s = sender.clone();
                let page_for_click = page.clone(); // <-- clone once per row
                row.connect_activated(move |_| {
                    s.input(AppModelInput::SelectPage(page_for_click.clone()));
                });
                list.append(&row);
            }

            let scrolled = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .child(&list)
                .build();

            library_nav.push(
                &adw::NavigationPage::builder()
                    .title(gettext("Library"))
                    .child(&scrolled)
                    .build(),
            );

            widgets.mobile_stack.add_titled_with_icon(
                &library_nav,
                Some("library"),
                &gettext("Library"),
                "library-symbolic",
            );
        }

        // Search (real)
        widgets.mobile_stack.add_titled_with_icon(
            model.search_page.widget(),
            Some("search"),
            &gettext("Search"),
            "edit-find-symbolic",
        );

        // Start on Home.
        widgets
            .mobile_stack
            .set_visible_child_name(&NavigationPage::Home.to_key());
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
            AppModelInput::ToggleSidebar => {
                self.is_sidebar_visible = !self.is_sidebar_visible;
            }
            AppModelInput::StartLoading => {
                self.is_loading = true;
            }
            AppModelInput::StopLoading => {
                self.is_loading = false;
            }

            AppModelInput::ToggleShowSearchPage(visible) => {
                self.is_search_page_visible = visible;
            }
            AppModelInput::SelectPage(page) => {
                let key = page.to_key();

                // 2. Lazily instantiate missing pages
                if !self.pages_cache.contains_key(&key) {
                    let controller = match page {
                        NavigationPage::Home => {
                            let page_instance = HomePage::builder().launch(()).forward(
                                sender.input_sender(),
                                |msg| match msg {
                                    HomPageOutput::Subscribe(feed) => {
                                        AppModelInput::Subscribe(feed)
                                    }
                                    HomPageOutput::ToggleSideBar => AppModelInput::ToggleSidebar,
                                    HomPageOutput::TogglePlay(episode_id) => {
                                        AppModelInput::TogglePlay(episode_id)
                                    }
                                    HomPageOutput::NotifyError(error) => {
                                        AppModelInput::NotifyError(error)
                                    }
                                    HomPageOutput::RequestDownload(episode_id) => {
                                        AppModelInput::RequestDownload(episode_id)
                                    }
                                    HomPageOutput::CancleDownload(episode_id) => {
                                        AppModelInput::CancleDownload(episode_id)
                                    }
                                    HomPageOutput::SetPlayNext(episode_id) => {
                                        AppModelInput::SetPlayNext(episode_id)
                                    }
                                    HomPageOutput::AddToPlaylist(episode_id) => {
                                        AppModelInput::AddToPlaylist(episode_id)
                                    }
                                    HomPageOutput::RequestDeleteEpisode(episode_id) => {
                                        AppModelInput::RequestDeleteEpisode(episode_id)
                                    }
                                    HomPageOutput::Execute(uuid, action) => {
                                        AppModelInput::ExecuteAction(uuid, action)
                                    }
                                },
                            );
                            PageController::Home(page_instance)
                        }
                        NavigationPage::New => {
                            let page_instance = NewPage::builder().launch(()).detach();
                            PageController::New(page_instance)
                        }
                        NavigationPage::Shows => {
                            let page_instance = ShowsPage::builder().launch(()).forward(
                                sender.input_sender(),
                                |msg| match msg {
                                    ShowsPageOutput::NotifyError(error) => {
                                        AppModelInput::NotifyError(error)
                                    }
                                    ShowsPageOutput::TogglePlay(id) => {
                                        AppModelInput::TogglePlay(id)
                                    }
                                    ShowsPageOutput::RequestDownload(episode_id) => {
                                        AppModelInput::RequestDownload(episode_id)
                                    }
                                    ShowsPageOutput::CancleDownload(episode_id) => {
                                        AppModelInput::CancleDownload(episode_id)
                                    }
                                    ShowsPageOutput::SetPlayNext(episode_id) => {
                                        AppModelInput::SetPlayNext(episode_id)
                                    }
                                    ShowsPageOutput::AddToPlaylist(episode_id) => {
                                        AppModelInput::AddToPlaylist(episode_id)
                                    }
                                    ShowsPageOutput::RequestDeleteEpisode(episode_id) => {
                                        AppModelInput::RequestDeleteEpisode(episode_id)
                                    }
                                    ShowsPageOutput::Execute(uuid, action) => {
                                        AppModelInput::ExecuteAction(uuid, action)
                                    }
                                },
                            );
                            PageController::Shows(page_instance)
                        }

                        NavigationPage::Downloads => {
                            let page_instance = DownloadsPage::builder().launch(()).forward(
                                sender.input_sender(),
                                |msg| match msg {
                                    DownloadsPageOutput::TogglePlay(episode_id) => {
                                        AppModelInput::TogglePlay(episode_id)
                                    }
                                    DownloadsPageOutput::NotifyError(error) => {
                                        AppModelInput::NotifyError(error)
                                    }
                                    DownloadsPageOutput::RequestDeleteEpisode(episode_id) => {
                                        AppModelInput::RequestDeleteEpisode(episode_id)
                                    }
                                    DownloadsPageOutput::Execute(uuid, action) => {
                                        AppModelInput::ExecuteAction(uuid, action)
                                    }
                                },
                            );
                            PageController::Downloads(page_instance)
                        }

                        NavigationPage::Recents => {
                            let page_instance = RecentlyUpdatedPage::builder().launch(()).forward(
                                sender.input_sender(),
                                |msg| match msg {
                                    RecentlyUpdatedPageOutput::TogglePlay(episode_id) => {
                                        AppModelInput::TogglePlay(episode_id)
                                    }
                                    RecentlyUpdatedPageOutput::NotifyError(error) => {
                                        AppModelInput::NotifyError(error)
                                    }
                                    RecentlyUpdatedPageOutput::RequestDownload(episode_id) => {
                                        AppModelInput::RequestDownload(episode_id)
                                    }
                                    RecentlyUpdatedPageOutput::CancleDownload(episode_id) => {
                                        AppModelInput::CancleDownload(episode_id)
                                    }
                                    RecentlyUpdatedPageOutput::SetPlayNext(episode_id) => {
                                        AppModelInput::SetPlayNext(episode_id)
                                    }
                                    RecentlyUpdatedPageOutput::AddToPlaylist(episode_id) => {
                                        AppModelInput::AddToPlaylist(episode_id)
                                    }

                                    RecentlyUpdatedPageOutput::RequestDeleteEpisode(episode_id) => {
                                        AppModelInput::RequestDeleteEpisode(episode_id)
                                    }
                                    RecentlyUpdatedPageOutput::ExcuteAction(uuid, action) => {
                                        AppModelInput::ExecuteAction(uuid, action)
                                    }
                                },
                            );
                            PageController::Recents(page_instance)
                        }
                        _ => return,
                    };

                    self.pages_cache.insert(key.clone(), controller);

                    // Swap the placeholder (New) or add fresh (Recents/Shows/Downloads).
                    // Both share the same key, so first remove any child under that name.
                    while let Some(existing) = widgets.mobile_stack.child_by_name(&key) {
                        widgets.mobile_stack.remove(&existing);
                    }
                    if let Some(c) = self.pages_cache.get(&key) {
                        widgets.mobile_stack.add_titled_with_icon(
                            c.widget(),
                            Some(&key),
                            &page.title(),
                            &page.icon_name(),
                        );
                    }
                }

                // 3. Update active page and UI view
                //self.current_page_key = key;
                self.current_page_key = key.clone();
                widgets.mobile_stack.set_visible_child_name(&key);
                // if let Some(cached_page) = self.pages_cache.get(&self.current_page_key) {
                //     widgets.content_bin.set_child(Some(cached_page.widget()));
                // }
            }

            AppModelInput::SetSidebarCollapsed(_is_collapsed) => {}

            AppModelInput::NotifyError(error) => {
                eprintln!("Error: {error}");
                let toast = adw::Toast::builder()
                    .title(error)
                    .action_name("Action")
                    .build();
                widgets.toast_overlay.add_toast(toast);
            }
            AppModelInput::Subscribe(feed) => {
                let id = uuid::Uuid::new_v4();
                self.worker_controller
                    .emit(ActionWorkerInput::Execute(id, Action::Subscribe(feed)));
            }
            AppModelInput::TogglePlay(episode_id) => {
                self.player
                    .emit(GStreamerWorkerInput::PlayEpisode(episode_id));
            }
            AppModelInput::ChangePlayBackState(state, episode_id) => {
                self.search_page.notify_playing_state(episode_id, state);
                self.miniplayer
                    .emit(MiniplayerModelInput::ChangePlayBackState(state));
                self.player_page
                    .emit(PlayerPageInput::ChangePlayBackState(state));
                for (_, page) in &self.pages_cache {
                    page.notify_playing_state(episode_id, state);
                }
            }
            AppModelInput::SetCurrentEpisode(id) => {
                self.search_page.notify_current_episode(id);
                self.miniplayer
                    .emit(MiniplayerModelInput::SetCurrentEpisode(id));
                self.player_page
                    .emit(PlayerPageInput::SetCurrentEpisode(id));
                for (_, page) in &self.pages_cache {
                    page.notify_current_episode(id);
                }
            }
            AppModelInput::TogglePlayBack => {
                self.player.emit(GStreamerWorkerInput::TogglePlayBack);
            }
            AppModelInput::RefreshShowsPage => {
                if let Some(PageController::Shows(shows_controller)) =
                    self.pages_cache.get(&self.current_page_key)
                {
                    shows_controller.sender().emit(ShowsPageInput::FetchShows);
                }
            }

            AppModelInput::None => {}
            AppModelInput::RequestDownload(episode_id) => {
                self.database
                    .emit(DatabaseWokerInput::DownloadEpisode(episode_id));
            }
            AppModelInput::CancleDownload(episode_id) => {
                self.database
                    .emit(DatabaseWokerInput::CancelDownload(episode_id));
            }
            AppModelInput::DownloadStarted(episode_id) => {
                self.search_page.notify_download_started(episode_id);
                for (_, page) in &self.pages_cache {
                    page.notify_download_started(episode_id);
                }
            }
            AppModelInput::DownloadCancled(episode_id) => {
                println!("Download of {:?} Cancled", episode_id);
            }
            AppModelInput::DownloadProgress(episode_id, fraction) => {
                self.search_page
                    .notify_download_progress(episode_id, fraction);
                for (_, page) in &self.pages_cache {
                    page.notify_download_progress(episode_id, fraction);
                }
            }
            AppModelInput::DownloadFinished(episode_id) => {
                self.search_page.notify_download_finished(episode_id);
                for (_, page) in &self.pages_cache {
                    page.notify_download_finished(episode_id);
                }
            }
            AppModelInput::PlayBackProgress(episode_id, pos, remaining) => {
                self.search_page
                    .notify_playback_progress(episode_id, pos, remaining);
                self.miniplayer
                    .emit(MiniplayerModelInput::UpdateProgress(pos));

                self.player_page
                    .emit(PlayerPageInput::UpdateProgress(pos, remaining));
                for (_, page) in &self.pages_cache {
                    page.notify_playback_progress(episode_id, pos, remaining);
                }
            }
            AppModelInput::SeekAudioPosition(fraction) => {
                self.player
                    .emit(GStreamerWorkerInput::SeekAudioPosition(fraction));
            }
            AppModelInput::SetVolume(fraction) => {
                self.player.emit(GStreamerWorkerInput::SetVolume(fraction));
            }
            AppModelInput::RequestMute => {
                self.player.emit(GStreamerWorkerInput::RequestMute);
            }
            AppModelInput::RequestUnmute => {
                self.player.emit(GStreamerWorkerInput::RequestUnmute);
            }
            AppModelInput::RequestVolumeValue => {
                self.player.emit(GStreamerWorkerInput::GetVolume);
            }
            AppModelInput::VolumeValue(val) => {
                self.miniplayer.emit(MiniplayerModelInput::VolumeValue(val));

                self.player_page.emit(PlayerPageInput::VolumeValue(val));
            }
            AppModelInput::Seekforward => {
                self.player.emit(GStreamerWorkerInput::SeekFoward);
            }
            AppModelInput::SeekBakward => {
                self.player.emit(GStreamerWorkerInput::SeekBackward);
            }
            AppModelInput::RequestDeleteEpisode(episode_id) => {
                self.database
                    .emit(DatabaseWokerInput::DeleteEpisode(episode_id));
            }
            AppModelInput::EpisodeDeleted(episode_id) => {
                for (_, page) in &self.pages_cache {
                    page.notify_episode_deleted(episode_id);
                }
                self.search_page.notify_episode_deleted(episode_id);
            }
            AppModelInput::ShowPlayerPage(_player_page_view) => {
                widgets.nav_view.push(self.player_page.widget());
            }
            AppModelInput::AddToPlaylist(episode_id) => {
                self.player
                    .emit(GStreamerWorkerInput::AddToPlaylist(episode_id));
            }
            AppModelInput::UpdatePlaylist(ids, pos) => {
                self.player_page
                    .emit(PlayerPageInput::UpdatePlaylist(ids, pos));
            }
            AppModelInput::SetPlayNext(episode_id) => {
                self.player
                    .emit(GStreamerWorkerInput::SetPlayNext(episode_id));
            }
            AppModelInput::SetPlayNow(episode_id) => {
                self.player
                    .emit(GStreamerWorkerInput::SetPlayNow(episode_id));
            }
            AppModelInput::RemoveFromPlayList(episode_id) => {
                self.player
                    .emit(GStreamerWorkerInput::RemoveFromPlayList(episode_id));
            }
            AppModelInput::ExecuteAction(uuid, action) => {
                self.worker_controller
                    .emit(ActionWorkerInput::Execute(uuid, action));
            }
            AppModelInput::ActionFinished(uuid, result) => {
                for (_, page) in &self.pages_cache {
                    page.notify_action_finished(uuid, result.clone());
                }
                self.search_page
                    .notify_action_finished(uuid, result.clone());
            }
            AppModelInput::RequestSearch(task_id, search_text, search_category) => {
                self.worker_controller.emit(ActionWorkerInput::Execute(
                    task_id,
                    Action::Search(search_text.clone(), search_category.clone()),
                ));
            }
            AppModelInput::NavigationBreakpointChanged(is_narrow) => {
                if is_narrow != self.is_narrow {
                    self.is_narrow = is_narrow;
                    self.compact_layout = is_narrow;
                }
            }
        }

        self.update_view(widgets, sender);
    }
}
