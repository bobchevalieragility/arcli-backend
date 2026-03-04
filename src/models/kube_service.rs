#[derive(Clone, Debug)]
pub struct KubeService {
    pub namespace: String,
    pub name: String,
    pub port: u16,
}

impl KubeService {
    pub fn new(namespace: String, name: String, port: u16) -> KubeService {
        KubeService { namespace, name, port }
    }
}
