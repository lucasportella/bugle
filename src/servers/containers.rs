use std::cmp::Ordering;
use std::ops::{Deref, Index};
use std::sync::Arc;

use regex::{Regex, RegexBuilder};

use super::{Mode, Region, Server};

pub trait Servers: Index<usize, Output = Server> + Send + Sync {
    fn len(&self) -> usize;
}

impl Servers for Vec<Server> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
}

#[derive(Clone)]
pub struct ServerList {
    servers: Arc<dyn Servers>,
}

impl Deref for ServerList {
    type Target = dyn Servers;
    fn deref(&self) -> &Self::Target {
        self.servers.deref()
    }
}

impl<S: Servers + 'static> From<S> for ServerList {
    fn from(servers: S) -> Self {
        Self {
            servers: Arc::new(servers),
        }
    }
}

impl ServerList {
    pub fn empty() -> Self {
        Self {
            servers: Arc::new(vec![]),
        }
    }

    pub fn sorted(&self, criteria: SortCriteria) -> Self {
        Self {
            servers: Arc::new(ServerListView::sorted_from(self.servers.clone(), criteria)),
        }
    }

    pub fn filtered(&self, filter: &Filter) -> Self {
        Self {
            servers: Arc::new(ServerListView::filtered_from(self.servers.clone(), filter)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Map,
    Mode,
    Region,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortCriteria {
    pub key: SortKey,
    pub ascending: bool,
}

impl SortCriteria {
    pub fn reversed(&self) -> Self {
        Self {
            key: self.key,
            ascending: !self.ascending,
        }
    }

    fn comparator(&self) -> Box<dyn FnMut(&Server, &Server) -> Ordering> {
        let cmp = match self.key {
            SortKey::Name => |lhs: &Server, rhs: &Server| lhs.name.cmp(&rhs.name),
            SortKey::Map => |lhs: &Server, rhs: &Server| lhs.map.cmp(&rhs.map),
            SortKey::Mode => |lhs: &Server, rhs: &Server| lhs.mode().cmp(&rhs.mode()),
            SortKey::Region => |lhs: &Server, rhs: &Server| lhs.region.cmp(&rhs.region),
        };
        let cmp = move |lhs: &Server, rhs: &Server| {
            cmp(lhs, rhs).then_with(|| Self::tie_breaker(lhs, rhs))
        };
        if self.ascending {
            Box::new(cmp)
        } else {
            Box::new(move |lhs, rhs| cmp(lhs, rhs).reverse())
        }
    }

    fn tie_breaker(lhs: &Server, rhs: &Server) -> Ordering {
        lhs.id.cmp(&rhs.id)
    }
}

#[derive(Clone, Debug)]
pub struct Filter {
    name: Regex,
    map: Regex,
    mode: Option<Mode>,
    region: Option<Region>,
    build_id: Option<u32>,
    password_protected: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Filter {
            name: Regex::new("").unwrap(),
            map: Regex::new("").unwrap(),
            mode: None,
            region: None,
            build_id: None,
            password_protected: false,
        }
    }
}

impl Filter {
    pub fn set_name(&mut self, name: String) {
        self.name = RegexBuilder::new(&regex::escape(&name))
            .case_insensitive(true)
            .build()
            .unwrap();
    }

    pub fn set_map(&mut self, map: String) {
        self.map = RegexBuilder::new(&regex::escape(&map))
            .case_insensitive(true)
            .build()
            .unwrap();
    }

    pub fn set_mode(&mut self, mode: impl Into<Option<Mode>>) {
        self.mode = mode.into();
    }

    pub fn set_region(&mut self, region: impl Into<Option<Region>>) {
        self.region = region.into();
    }

    pub fn set_build_id(&mut self, build_id: impl Into<Option<u32>>) {
        self.build_id = build_id.into();
    }

    pub fn set_password_protected(&mut self, password_protected: bool) {
        self.password_protected = password_protected;
    }

    pub fn matches(&self, server: &Server) -> bool {
        self.name.is_match(&server.name)
            && self.map.is_match(&server.map)
            && self.mode.map_or(true, |mode| server.mode() == mode)
            && self.region.map_or(true, |region| server.region == region)
            && self.build_id.map_or(true, |id| server.build_id == id)
            && self.password_protected >= server.password_protected
    }
}

struct ServerListView {
    source: Arc<dyn Servers>,
    indices: Vec<usize>,
}

impl ServerListView {
    fn sorted_from(source: Arc<dyn Servers>, criteria: SortCriteria) -> Self {
        let mut indices: Vec<usize> = (0..source.len()).collect();
        let mut comparator = criteria.comparator();
        indices.sort_unstable_by(|lidx, ridx| comparator(&source[*lidx], &source[*ridx]));
        Self { source, indices }
    }

    fn filtered_from(source: Arc<dyn Servers>, filter: &Filter) -> Self {
        let indices: Vec<usize> = (0..source.len())
            .into_iter()
            .filter(|idx| filter.matches(&source[*idx]))
            .collect();
        Self { source, indices }
    }
}

impl Index<usize> for ServerListView {
    type Output = Server;
    fn index(&self, index: usize) -> &Self::Output {
        &self.source[self.indices[index]]
    }
}

impl Servers for ServerListView {
    fn len(&self) -> usize {
        self.indices.len()
    }
}

impl<'l> IntoIterator for &'l dyn Servers {
    type Item = &'l Server;
    type IntoIter = ServerIter<'l>;

    fn into_iter(self) -> Self::IntoIter {
        ServerIter::new(self)
    }
}

pub struct ServerIter<'l> {
    list: &'l dyn Servers,
    idx: usize,
}

impl<'l> ServerIter<'l> {
    fn new(list: &'l dyn Servers) -> Self {
        Self { list, idx: 0 }
    }
}

impl<'l> Iterator for ServerIter<'l> {
    type Item = &'l Server;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.list.len() {
            return None;
        }

        let result = &self.list[self.idx];
        self.idx += 1;
        Some(result)
    }
}
