use std::{
    cell::{RefCell, RefMut},
    iter::Peekable,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FileType {
    Text,
    #[default]
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct File {
    name: String,
    content: Vec<u8>, // max 1000 bytes, rest of the file truncated
    creation_time: u64,
    type_: FileType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dir {
    name: String,
    creation_time: u64,
    children: Vec<Rc<RefCell<Node>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    File(File),
    Dir(Dir),
}

#[derive(Debug, Clone)]
pub struct FileSystem {
    root: Rc<RefCell<Dir>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MatchResult<'a> {
    queries: Vec<&'a str>, // query matchated
    nodes: Vec<Rc<RefCell<Node>>>,
}

#[derive(Debug, Clone)]
enum QueryParam {
    Name(String, usize),
    Content(String, usize),
    Larger(u32, usize),
    Smaller(u32, usize),
    Newer(u64, usize),
    Older(u64, usize),
}

impl QueryParam {
    fn match_value(&self, node: &Node) -> bool {
        match self {
            Self::Name(name, _) => node.get_name().contains(name),
            Self::Content(content, _) => match node.get_content() {
                None => false,
                Some(c) => String::from_utf8(c.to_vec()).map_or(false, |s| s.contains(content)),
            },
            Self::Larger(size, _) => node.get_size().map_or(false, |s| s > *size),
            Self::Smaller(size, _) => node.get_size().map_or(false, |s| s < *size),
            Self::Newer(time, _) => node.get_creation_time() > *time,
            Self::Older(time, _) => node.get_creation_time() < *time,
        }
    }

    fn match_dir(&self, dir: &Dir) -> bool {
        match self {
            Self::Name(name, _) => dir.name == *name,
            Self::Newer(time, _) => dir.creation_time > *time,
            Self::Older(time, _) => dir.creation_time < *time,
            _ => false,
        }
    }

    fn match_file(&self, file: &File) -> bool {
        match self {
            Self::Name(name, _) => file.name == *name,
            Self::Content(content, _) => {
                String::from_utf8(file.content.to_vec()).map_or(false, |s| s.contains(content))
            }
            Self::Larger(size, _) => file.content.len() > (*size as usize),
            Self::Smaller(size, _) => file.content.len() < (*size as usize),
            Self::Newer(time, _) => file.creation_time > *time,
            Self::Older(time, _) => file.creation_time < *time,
        }
    }

    fn get_index(&self) -> usize {
        match self {
            Self::Name(_, i) => *i,
            Self::Content(_, i) => *i,
            Self::Larger(_, i) => *i,
            Self::Smaller(_, i) => *i,
            Self::Newer(_, i) => *i,
            Self::Older(_, i) => *i,
        }
    }
}

impl Node {
    fn get_name(&self) -> &str {
        match self {
            Self::Dir(d) => &d.name,
            Self::File(f) => &f.name,
        }
    }

    fn get_content(&self) -> Option<&Vec<u8>> {
        match self {
            Self::Dir(_) => None,
            Self::File(f) => Some(&f.content),
        }
    }

    fn get_size(&self) -> Option<u32> {
        match self {
            Self::Dir(_) => None,
            Self::File(f) => Some(f.content.len() as u32),
        }
    }

    fn get_creation_time(&self) -> u64 {
        match self {
            Self::Dir(d) => d.creation_time,
            Self::File(f) => f.creation_time,
        }
    }

    fn match_queries(&mut self, queries: &mut Vec<(QueryParam, bool)>) -> bool {
        let mut query_matched = false;

        for query in queries.iter_mut() {
            if query.0.match_value(self) {
                query.1 = true;
                query_matched = true;
            }
        }

        return query_matched;
    }

    fn children_len(&self) -> usize {
        match self {
            Self::Dir(d) => d.children.len(),
            _ => 0,
        }
    }

    fn is_file(&self) -> bool {
        match self {
            Self::File(_) => true,
            _ => false,
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            Self::Dir(_) => true,
            _ => false,
        }
    }

    fn as_dir(&mut self) -> Option<&mut Dir> {
        match self {
            Self::Dir(d) => Some(d),
            _ => None,
        }
    }

    fn as_file(&mut self) -> Option<&mut File> {
        match self {
            Self::File(f) => Some(f),
            _ => None,
        }
    }
}

fn creation_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

impl Dir {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            creation_time: creation_time(),
            children: vec![],
        }
    }

    fn get_child_mut(&self, index: usize) -> Option<RefMut<Node>> {
        self.children
            .get(index)
            .map(|node| node.as_ref().borrow_mut())
    }

    fn mk_dir<'a>(&mut self, path: &mut Peekable<impl Iterator<Item = &'a str>>) {
        let next = match path.next() {
            None => {
                return;
            }
            Some(val) => val,
        };

        // next is last path
        if path.peek().is_none() {
            if self.contains_mut(next).is_none() {
                self.children
                    .push(Rc::new(RefCell::new(Node::Dir(Dir::new(next)))));
                return;
            }
            return;
        }

        if let Some(node) = self.contains_mut(next) {
            let mut dir = node.as_ref().borrow_mut();
            if let Node::Dir(ref mut next_dir) = *dir {
                next_dir.mk_dir(path);
            }
        }
    }

    fn rm_dir<'a>(&mut self, path: &mut Peekable<impl Iterator<Item = &'a str>>) {
        let next = match path.next() {
            None => {
                return;
            }
            Some(val) => val,
        };

        // curr is last path
        if path.peek().is_none() {
            let index = {
                let index_maybe = self
                    .children
                    .iter()
                    .position(|c| c.borrow().get_name() == next);

                let index = match index_maybe {
                    None => return,
                    Some(val) => val,
                };

                if let Node::Dir(ref dir_to_remove) = *self.children[index].borrow() {
                    if dir_to_remove.children.len() != 0 {
                        return;
                    }
                }

                index
            };

            self.children.remove(index);
            return;
        }

        if let Some(node) = self.contains_mut(next) {
            if let Node::Dir(ref mut next_dir) = *node.as_ref().borrow_mut() {
                next_dir.rm_dir(path);
            }
        }
    }

    fn new_file<'a>(
        &mut self,
        path: &mut Peekable<impl Iterator<Item = &'a str>>,
        file: File,
    ) -> bool {
        let curr = match path.next() {
            Some(n) => n,
            None => return false,
        };

        if self.name != curr {
            return false;
        }

        if path.peek().is_none() && self.contains_file(&file.name).is_none() {
            self.children.push(Rc::new(RefCell::new(Node::File(file))));
            return true;
        }

        if let Some(dir) = self.contains_dir(path.peek().unwrap()) {
            return dir
                .as_ref()
                .borrow_mut()
                .as_dir()
                .unwrap()
                .new_file(path, file);
        }

        return false;
    }

    fn contains_mut(&mut self, name: &str) -> Option<Rc<RefCell<Node>>> {
        let mut iter = self.children.iter();

        let res = iter.find(|n| match *n.borrow() {
            Node::File(ref f) => f.name == name,
            Node::Dir(ref d) => d.name == name,
        });

        res.map(|node| node.clone())
    }

    fn contains_file(&mut self, name: &str) -> Option<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .find(|child| match *child.borrow() {
                Node::File(ref f) => f.name == name,
                _ => false,
            })
            .map_or(None, |file| match *file.as_ref().borrow() {
                Node::File(_) => Some(file.clone()),
                _ => None,
            })
    }

    fn contains_dir(&mut self, name: &str) -> Option<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .find(|child| match *child.borrow() {
                Node::Dir(ref d) => d.name == name,
                _ => false,
            })
            .map_or(None, |dir| match *dir.as_ref().borrow() {
                Node::Dir(_) => Some(dir.clone()),
                _ => None,
            })
    }

    fn remove(&mut self, name: &str) {
        let pos = match self.children.iter().position(|c| match *c.borrow() {
            Node::File(ref f) => f.name == name,
            Node::Dir(_) => false,
        }) {
            Some(p) => p,
            None => return,
        };

        self.children.remove(pos);
    }

    fn match_queries(&mut self, queries: &mut Vec<(QueryParam, bool)>) -> bool {
        let mut query_matched = false;

        for query in queries.iter_mut() {
            if query.0.match_dir(self) {
                query.1 = true;
                query_matched = true;
            }
        }

        return query_matched;
    }

    fn query(&mut self, queries: &mut Vec<(QueryParam, bool)>) -> Vec<Rc<RefCell<Node>>> {
        let mut nodes = vec![];

        nodes.extend(self.children.iter().flat_map(|c| {
            let mut matches = vec![];
            if c.borrow_mut().match_queries(queries) {
                matches.push(c.clone());
            }

            if let Node::Dir(ref mut dir) = *c.borrow_mut() {
                matches.extend(dir.query(queries));
            }

            matches
        }));

        nodes
    }
}

impl Into<Node> for Dir {
    fn into(self) -> Node {
        Node::Dir(self)
    }
}

impl File {
    fn match_queries(&mut self, queries: &mut Vec<(QueryParam, bool)>) -> bool {
        let mut query_matched = false;

        for query in queries.iter_mut() {
            if query.0.match_file(self) {
                query.1 = true;
                query_matched = true;
            }
        }

        return query_matched;
    }
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            root: Rc::new(RefCell::new(Dir::new(""))),
        }
    }

    pub fn from_dir(path: &str) {}

    pub fn mk_dir(&mut self, path: &str) {
        let iter = &mut path.split("/").peekable();

        let mut root = self.root.as_ref().borrow_mut();

        if let Some(next) = iter.next() {
            if next != root.name {
                return;
            }

            root.mk_dir(iter);
        }
    }

    pub fn rm_dir(&mut self, path: &str) {
        let iter = &mut path.split("/").peekable();

        let mut root = self.root.as_ref().borrow_mut();
        if let Some(next) = iter.next() {
            if next != root.name {
                return;
            }

            root.rm_dir(iter);
        }
    }

    pub fn new_file(&mut self, path: &str, file: File) -> bool {
        let mut dirs = path.trim().split_terminator("/").peekable();
        self.root.as_ref().borrow_mut().new_file(&mut dirs, file)
    }

    pub fn get_file(&mut self, path: &str) -> Option<Rc<RefCell<Node>>> {
        let mut split_path = path.split("/");
        if split_path.next() != Some("") {
            return None;
        }

        // go through all the paths
        let split_path: Vec<&str> = split_path.collect();

        let mut curr_dir = match self
            .root
            .as_ref()
            .borrow_mut()
            .contains_dir(split_path.first().unwrap())
        {
            None => return None,
            Some(dir) => dir,
        };

        for file in split_path[1..split_path.len() - 1].iter() {
            let new_dir = if let Some(node) = curr_dir
                .borrow_mut()
                .as_dir()
                .and_then(|d| d.contains_mut(file))
            {
                match *node.borrow() {
                    Node::Dir(_) => node.clone(),
                    Node::File(_) => return None,
                }
            } else {
                return None;
            };

            curr_dir = new_dir;
        }

        if let Some(p) = split_path.last() {
            if let Some(file) = curr_dir
                .borrow_mut()
                .as_dir()
                .and_then(|d| d.contains_mut(p))
            {
                return match *file.borrow() {
                    Node::File(_) => Some(file.clone()),
                    _ => None,
                };
            }
        }

        return None;
    }

    pub fn search<'a>(&mut self, queries: &[&'a str]) -> Option<MatchResult<'a>> {
        let mut result = MatchResult {
            queries: vec![],
            nodes: vec![],
        };

        let mut final_queries: Vec<(QueryParam, bool)> = vec![];
        // build vec of query
        for (index, query) in queries
            .iter()
            .map(|q| q.split(":").collect::<Vec<&str>>())
            .enumerate()
        {
            if query.len() != 2 {
                return None;
            }

            let final_query = match query[0] {
                "name" => QueryParam::Name(query[1].to_string(), index),
                "content" => QueryParam::Content(query[1].to_string(), index),
                "larger" => QueryParam::Larger(
                    match query[1].to_string().parse::<u32>() {
                        Ok(l) => l,
                        Err(_) => return None,
                    },
                    index,
                ),
                "smaller" => QueryParam::Smaller(
                    match query[1].to_string().parse::<u32>() {
                        Ok(l) => l,
                        Err(_) => return None,
                    },
                    index,
                ),
                "newer" => QueryParam::Newer(
                    match query[1].to_string().parse::<u64>() {
                        Ok(l) => l,
                        Err(_) => return None,
                    },
                    index,
                ),
                "older" => QueryParam::Older(
                    match query[1].to_string().parse::<u64>() {
                        Ok(l) => l,
                        Err(_) => return None,
                    },
                    index,
                ),
                _ => return None,
            };

            final_queries.push((final_query, false));
        }

        let nodes = self.root.borrow_mut().query(&mut final_queries);

        result.nodes = nodes;
        result.queries = final_queries
            .into_iter()
            .filter(|fq| fq.1 == true)
            .map(|fq| queries[fq.0.get_index()])
            .collect();

        Some(result)
    }
}

#[cfg(test)]
mod test {

    use crate::{File, FileSystem, Node};

    #[test]
    fn new_test() {
        let file = FileSystem::new();

        let root = file.root.as_ref().borrow();
        assert_eq!("", root.name);
        assert_eq!(0, root.children.len());
        assert_ne!(0, root.creation_time);
    }

    #[test]
    fn mk_dir_test() {
        let mut file = FileSystem::new();
        file.mk_dir("/a");
        file.mk_dir("/b");
        file.mk_dir("/a/c");
        file.mk_dir("/a/d");

        let children = &file.root.as_ref().borrow_mut().children;
        assert_eq!("a", children[0].as_ref().borrow().get_name());
        assert_eq!("b", children[1].as_ref().borrow().get_name());

        assert_eq!(
            "c",
            children[0].as_ref().borrow_mut().as_dir().unwrap().children[0]
                .as_ref()
                .borrow()
                .get_name()
        );
        assert_eq!(
            "d",
            children[0].as_ref().borrow_mut().as_dir().unwrap().children[1]
                .as_ref()
                .borrow()
                .get_name()
        );
    }

    #[test]
    fn rm_dir_test() {
        let mut file = FileSystem::new();
        file.mk_dir("/a");
        file.mk_dir("/b");
        file.mk_dir("/a/c");
        file.mk_dir("/a/d");

        file.rm_dir("/a/c");
        {
            let root = file.root.as_ref().borrow();
            assert_eq!(
                1,
                root.get_child_mut(0)
                    .unwrap()
                    .as_dir()
                    .map_or(0, |c| c.children.len())
            );
        }

        file.rm_dir("/a/f");
        {
            let root = file.root.as_ref().borrow();
            assert_eq!(
                1,
                root.get_child_mut(0)
                    .unwrap()
                    .as_dir()
                    .map_or(0, |c| c.children.len())
            );
        }

        file.rm_dir("/a/d");
        {
            let root = file.root.as_ref().borrow();
            assert_eq!(
                0,
                root.get_child_mut(0)
                    .unwrap()
                    .as_dir()
                    .map_or(0, |c| c.children.len())
            );
        }
    }

    #[test]
    fn new_file_test() {
        let mut file = FileSystem::new();
        file.mk_dir("/a");
        file.mk_dir("/b");
        file.mk_dir("/a/c");
        file.mk_dir("/a/d");

        let new_file = File {
            name: "Sium".to_string(),
            content: vec![0, 1, 2],
            creation_time: 0,
            type_: crate::FileType::Binary,
        };

        assert!(file.new_file("/", new_file.clone()));
        {
            let root = file.root.as_ref().borrow();
            assert_eq!(
                Node::File(new_file.clone()),
                *root.children[2].as_ref().borrow()
            );
        }

        assert!(file.new_file("/a", new_file.clone()));
        {
            let root = file.root.as_ref().borrow();
            assert_eq!(
                Node::File(new_file.clone()),
                *root.children[0]
                    .as_ref()
                    .borrow_mut()
                    .as_dir()
                    .unwrap()
                    .children[2]
                    .as_ref()
                    .borrow()
            );
        }
    }

    #[test]
    fn search_test() {
        let mut file = FileSystem::new();
        file.new_file(
            "/",
            File {
                name: "a".into(),
                ..Default::default()
            },
        );
        file.mk_dir("/b");
        file.mk_dir("/b/c");
        file.mk_dir("/b/d");
        file.mk_dir("/b/c/a");
        file.new_file(
            "/b/d",
            File {
                name: "o".into(),
                ..Default::default()
            },
        );

        let matches = file
            .search(&["name:a", "name:f", "name:o", "smaller:32"])
            .unwrap();

        assert_eq!(matches.queries.len(), 3);
        assert_eq!(matches.nodes.len(), 3);
    }
}
