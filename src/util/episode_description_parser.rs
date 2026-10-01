// episode_description.rs
//
// Copyright 2021 nee <nee-git@patchouli.garden>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use formatx::formatx;
use gettextrs::gettext;
use linkify::LinkFinder;
use linkify::LinkKind;
use regex::Regex;

use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{ParseOpts, expanded_name, local_name, ns, parse_document};
use markup5ever_rcdom::{
    Handle,
    NodeData::{Document, Element, Text},
    RcDom,
};

const INDENT: i32 = 4; // used by li tags

#[derive(Clone)]
enum NewlineHandling {
    ToSpace,
    Remove,
    Keep,
}

fn escape_text(t: &str, newline_handling: NewlineHandling) -> String {
    let escaped = t
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        // prevent escaping escape-sequances
        .replace("&amp;amp;", "&amp;")
        .replace("&lt;lt;", "&lt;")
        .replace("&gt;gt;", "&gt;");
    let newlined_text = match newline_handling {
        NewlineHandling::ToSpace => escaped.replace('\n', " "),
        NewlineHandling::Remove => escaped.replace('\n', ""),
        NewlineHandling::Keep => escaped,
    };
    collapse_whitespaces(newlined_text)
}

// remove spaces that follow on another space or a newline
fn collapse_whitespaces(string: String) -> String {
    let mut was_space = false;
    string
        .chars()
        .filter(|c| {
            let is_space = c.eq(&' ');
            if is_space && was_space {
                return false;
            }
            was_space = is_space || c.eq(&'\n');
            true
        })
        .collect()
}

// Does the description use \n Text newlines or <br> <p> Tag newlines
#[derive(Debug)]
enum NewlineStyle {
    Text,
    Tag,
}
enum ListStyle {
    Ordered(i32),
    Unordered,
}
struct ParserState {
    nl_style: NewlineStyle,
    skip_leading_spaces: bool,
    indent: i32,
    list_style: Vec<ListStyle>,
    inside_link: i32,
}

fn find_newline_style(node: &Handle) -> NewlineStyle {
    match &node.data {
        Document => {
            let children = node.children.borrow();
            for el in children.iter() {
                if let NewlineStyle::Tag = find_newline_style(el) {
                    return NewlineStyle::Tag;
                }
            }
        }
        Element { name, .. } => {
            match name.expanded() {
                expanded_name!(html "p") => {
                    return NewlineStyle::Tag;
                }
                expanded_name!(html "br") => {
                    return NewlineStyle::Tag;
                }
                _ => (),
            };
            let children = node.children.borrow();
            for el in children.iter() {
                if let NewlineStyle::Tag = find_newline_style(el) {
                    return NewlineStyle::Tag;
                }
            }
        }
        _ => (),
    }
    NewlineStyle::Text
}

fn handle_child(buffer: &mut String, node: &Handle, state: &mut ParserState) {
    match &node.data {
        Document => {
            let children = node.children.borrow();
            for el in children.iter() {
                handle_child(buffer, el, state);
            }
        }
        Element { name, attrs, .. } => {
            let mut wrapper_href = None;
            let mut is_p_tag = false;
            let mut is_list_tag = false;
            let wrapper_tag = match name.expanded() {
                // Supported tags in pango markup
                // https://docs.gtk.org/Pango/pango_markup.html
                expanded_name!(html "a") => {
                    let local_name = local_name!("href");
                    wrapper_href = attrs
                        .borrow()
                        .iter()
                        .find(|attr| attr.name.local == local_name)
                        .cloned();
                    // Pango does not support a tags without href,
                    // so return a None in that case
                    wrapper_href.as_ref().map(|_| "a")
                }
                expanded_name!(html "p") => {
                    is_p_tag = true;
                    None
                }
                expanded_name!(html "br") => {
                    buffer.push('\n');
                    state.skip_leading_spaces = true;
                    None
                }
                expanded_name!(html "img") => {
                    let local_name = local_name!("alt");
                    let alt = attrs
                        .borrow()
                        .iter()
                        .find(|attr| attr.name.local == local_name)
                        .cloned();
                    if let Some(alt_text) = alt {
                        let escaped_alt = escape_text(&alt_text.value, NewlineHandling::ToSpace);
                        buffer.push('[');
                        buffer.push_str(escaped_alt.as_str());
                        buffer.push_str("]\n");
                        state.skip_leading_spaces = true;
                    }
                    None
                }
                expanded_name!(html "ol") => {
                    state.list_style.push(ListStyle::Ordered(1));
                    state.indent += INDENT;
                    is_list_tag = true;
                    is_p_tag = true;
                    None
                }
                expanded_name!(html "ul") => {
                    state.list_style.push(ListStyle::Unordered);
                    state.indent += INDENT;
                    is_list_tag = true;
                    is_p_tag = true;
                    None
                }
                expanded_name!(html "li") => {
                    buffer.push('\n');
                    for _ in 0..state.indent {
                        buffer.push(' ');
                    }
                    if let Some(style) = state.list_style.last_mut() {
                        match style {
                            ListStyle::Unordered => buffer.push_str("• "),
                            ListStyle::Ordered(i) => {
                                buffer.push_str(&format!("{}. ", i));
                                *style = ListStyle::Ordered(*i + 1);
                            }
                        }
                    }
                    state.skip_leading_spaces = true;
                    None
                }
                expanded_name!(html "b") => Some("b"),
                expanded_name!(html "i") => Some("i"),
                expanded_name!(html "s") => Some("s"),
                expanded_name!(html "u") => Some("u"),
                expanded_name!(html "tt") => Some("tt"),
                expanded_name!(html "pre") => Some("tt"),
                expanded_name!(html "code") => Some("tt"),
                expanded_name!(html "sub") => Some("sub"),
                expanded_name!(html "sup") => Some("sup"),
                _ => None,
            };
            // Invalid link tag, links that point to # lead nowhere, skip the tag.
            let skip_tag = if let Some(href) = wrapper_href.as_ref() {
                wrapper_tag == Some("a")
                    && (href.value.trim_start().starts_with('#')
                        || href.value.trim().is_empty()
                        || href.value.trim_start().starts_with("jump:"))
            } else {
                false
            };
            let wrote_tag = if skip_tag {
                false
            } else if let Some(tag) = wrapper_tag {
                buffer.push('<');
                buffer.push_str(tag);
                let is_link;
                if let Some(href) = wrapper_href {
                    buffer.push_str(" href=\"");
                    buffer.push_str(&escape_text(&href.value, NewlineHandling::Remove));
                    buffer.push('"');
                    state.inside_link += 1;
                    is_link = true;
                } else {
                    is_link = false;
                }

                buffer.push('>');

                let children = node.children.borrow();
                for el in children.iter() {
                    handle_child(buffer, el, state);
                }
                buffer.push_str("</");
                buffer.push_str(tag);
                buffer.push('>');

                if is_link {
                    state.inside_link -= 1;
                }
                true
            } else {
                false
            };

            if !wrote_tag {
                let children = node.children.borrow();
                for el in children.iter() {
                    handle_child(buffer, el, state);
                }
                if is_p_tag {
                    buffer.push_str("\n\n");
                    state.skip_leading_spaces = true;
                }
                if is_list_tag {
                    state.indent -= INDENT;
                    state.list_style.pop();
                }
            }
        }
        Text { contents } => {
            let nl_handling = match state.nl_style {
                NewlineStyle::Tag => NewlineHandling::ToSpace,
                NewlineStyle::Text => NewlineHandling::Keep,
            };

            if state.skip_leading_spaces {
                let text = escape_text(contents.borrow().trim_start(), nl_handling.clone());
                if !text.is_empty() {
                    state.skip_leading_spaces = false;
                }
                if state.inside_link > 0 {
                    // avoid nested links
                    push_remaining_text(buffer, &text)
                } else {
                    push_timestamped_text(buffer, &text, nl_handling)
                }
            } else {
                let text = escape_text(&contents.borrow(), nl_handling.clone());
                if state.inside_link > 0 {
                    // avoid nested links
                    push_remaining_text(buffer, &text)
                } else {
                    push_timestamped_text(buffer, &text, nl_handling)
                }
            }
        }
        _ => (),
    }
}

fn push_timestamped_text(buffer: &mut String, text: &str, nl_handling: NewlineHandling) {
    let mut position = 0;
    if let Ok(re) = Regex::new(r"([0-9]+):([0-9]+)(?::([0-9]+))?") {
        for captures in re.captures_iter(text) {
            let first: Option<i32> = captures.get(1).and_then(|c| c.as_str().parse().ok());
            let second: Option<i32> = captures.get(2).and_then(|c| c.as_str().parse().ok());
            let third: Option<i32> = captures.get(3).and_then(|c| c.as_str().parse().ok());
            if let (Some(hours), Some(minutes), Some(seconds)) = (first, second, third) {
                let jump_time = (hours * 60 * 60) + (minutes * 60) + seconds;
                // Jump to Hours:Minutes:Seconds
                let localized_msg = gettext("Jump to {hours}:{minutes}:{seconds}");
                let h_str = format!("{:02}", hours);
                let m_str = format!("{:02}", minutes);
                let s_str = format!("{:02}", seconds);

                let title = formatx!(
                    &localized_msg,
                    hours = h_str,
                    minutes = m_str,
                    seconds = s_str,
                )
                .expect("Could not format translatable string");
                let range = captures.get(0).unwrap().range();

                push_text_with_links(buffer, &text[position..range.start], nl_handling.clone());
                buffer
                    .push_str(format!("<a href=\"jump:{jump_time}\" title=\"{title}\">").as_str());
                buffer.push_str(&text[range.start..range.end]);
                buffer.push_str("</a>");
                position = range.end;
            } else if let (Some(minutes), Some(seconds)) = (first, second) {
                let jump_time = (minutes * 60) + seconds;
                // Jump to Minutes:Seconds
                let localized_msg = gettext("Jump to {minutes}:{seconds}");
                let m_str = format!("{:02}", minutes);
                let s_str = format!("{:02}", seconds);
                let title = formatx!(&localized_msg, minutes = m_str, seconds = s_str,)
                    .expect("Could not format translatable string");
                let range = captures.get(0).unwrap().range();

                push_text_with_links(buffer, &text[position..range.start], nl_handling.clone());
                buffer.push_str(
                    format!("<a href=\"jump:{}\" title=\"{title}\">", jump_time).as_str(),
                );
                buffer.push_str(&text[range.start..range.end]);
                buffer.push_str("</a>");
                position = range.end;
            }
        }
        push_text_with_links(buffer, &text[position..], nl_handling);
    } else {
        push_text_with_links(buffer, text, nl_handling);
    }
}

fn push_text_with_links(buffer: &mut String, text: &str, nl_handling: NewlineHandling) {
    let mut finder = LinkFinder::new();
    finder.url_must_have_scheme(false);
    let mut position = 0;
    for link in finder.links(text) {
        let link_str = link.as_str();
        let remaining_link_text = escape_text(&text[position..link.start()], nl_handling.clone());
        push_remaining_text(buffer, &remaining_link_text);

        match link.kind() {
            LinkKind::Email => {
                buffer.push_str(format!("<a href=\"mailto:{}\">", link_str).as_str());
            }
            LinkKind::Url => {
                if link.as_str().starts_with("http://") || link_str.starts_with("https://") {
                    buffer.push_str(format!("<a href=\"{}\">", link_str).as_str());
                } else {
                    buffer.push_str(format!("<a href=\"https:{}\">", link_str).as_str());
                }
            }
            _ => {
                buffer.push_str(format!("<a href=\"{}\">", link_str).as_str());
            }
        }
        push_remaining_text(buffer, &text[(link.start())..(link.end())]);
        buffer.push_str("</a>");
        position = link.end();
    }

    let end_text = escape_text(&text[position..], nl_handling);
    push_remaining_text(buffer, &end_text);
}

pub fn push_remaining_text(buffer: &mut String, text: &str) {
    // start adding new plaintext replacements here
    buffer.push_str(text);
}

pub fn html2pango_markup(t: &str) -> String {
    let mut buffer = String::with_capacity(t.len());
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let dom: RcDom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut t.as_bytes())
        .unwrap();

    let root: Handle = dom.document;
    let nl_style = find_newline_style(&root);
    handle_child(
        &mut buffer,
        &root,
        &mut ParserState {
            nl_style,
            skip_leading_spaces: true,
            indent: 0,
            list_style: vec![],
            inside_link: 0,
        },
    );
    buffer
}
