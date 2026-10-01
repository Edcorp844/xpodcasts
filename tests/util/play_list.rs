use podcasts_data::EpisodeId;
use xpodcasts::util::play_list::PlayList;

fn id(n: i32) -> EpisodeId {
    EpisodeId::from(podcasts_data::EpisodeId(n))
}

#[test]
fn new_playlist_is_empty() {
    let playlist = PlayList::new();
    assert!(playlist.is_empty());
    assert_eq!(playlist.current(), None);
}

#[test]
fn push_back_sets_current_to_first_item() {
    let mut playlist = PlayList::new();
    playlist.push_back(id(1));
    assert_eq!(playlist.current(), Some(id(1)));
    assert_eq!(playlist.len(), 1);
}

#[test]
fn push_back_does_not_duplicate() {
    let mut playlist = PlayList::new();
    playlist.push_back(id(1));
    playlist.push_back(id(1));
    assert_eq!(playlist.len(), 1);
}

#[test]
fn next_and_prev_navigate_correctly() {
    let mut playlist = PlayList::new();
    playlist.push_back(id(1));
    playlist.push_back(id(2));
    playlist.push_back(id(3));

    assert_eq!(playlist.current(), Some(id(1)));
    assert_eq!(playlist.next(), Some(id(2)));

    playlist.set_current(playlist.clone().next().unwrap_or(id(0)));
    assert_eq!(playlist.next(), Some(id(3)));

    playlist.set_current(id(3));
    assert_eq!(playlist.len(), 3);
    assert_eq!(playlist.next(), None); //at end

    playlist.set_current(playlist.clone().prev().unwrap_or(id(0)));
    assert_eq!(playlist.current(), Some(id(2)));

    playlist.set_current(playlist.clone().prev().unwrap_or(id(0)));
    assert_eq!(playlist.prev(), None); // at start
}

#[test]
fn set_sequence_finds_starting_id() {
    let mut playlist = PlayList::new();
    playlist.set_sequence(vec![id(1), id(2), id(3)], &id(2));
    assert_eq!(playlist.current(), Some(id(2)));
    assert_eq!(playlist.next(), Some(id(3)));
}

#[test]
fn set_sequence_falls_back_to_first_when_starting_id_missing() {
    let mut playlist = PlayList::new();
    playlist.set_sequence(vec![id(1), id(2)], &id(99));
    assert_eq!(playlist.current(), Some(id(1)));
}

#[test]
fn set_sequence_with_empty_list_has_no_current() {
    let mut playlist = PlayList::new();
    playlist.set_sequence(vec![], &id(1));
    assert_eq!(playlist.current(), None);
    assert!(playlist.is_empty());
}

#[test]
fn remove_current_item() {
    let mut playlist = PlayList::new();
    playlist.set_sequence(vec![id(1), id(2), id(3)], &id(2));
    playlist.remove(&id(2));
    assert_eq!(playlist.current(), None);
    assert_eq!(playlist.len(), 2);
}

#[test]
fn remove_last_remaining_item_clears_current() {
    let mut playlist = PlayList::new();
    playlist.set_sequence(vec![id(1)], &id(1));
    playlist.remove(&id(1));
    assert_eq!(playlist.current(), None);
    assert!(playlist.is_empty());
}
