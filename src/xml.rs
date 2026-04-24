use super::*;
use xmlwriter::{Options, XmlWriter};

pub struct XmlCtx<'ctx> {
    ctx: &'ctx Context,
    xml: XmlWriter,
}

impl<'ctx> XmlCtx<'ctx> {
    pub fn write_node(&mut self, id: id::AnyNode) {
        let node = &self.ctx.node(id);

        self.xml.start_element("node");

        self.xml.write_attribute("id", &node.id);
        if let Some(name) = self.ctx.symbols.get(id) {
            self.xml.write_attribute("name", &name);
        }
        self.xml.write_attribute("type", node.kind.node_type());

        for i in self.ctx.inputs(id) {
            self.xml.start_element("input");
            self.xml.write_attribute("id", &i);
            self.xml.end_element();
        }

        for o in self.ctx.outputs(id) {
            self.xml.start_element("output");
            self.xml.write_attribute("id", &o);
            self.xml.end_element();
        }

        for region in node.regions.as_slice(&self.ctx.region_id_pool) {
            self.write_region(*region);
        }

        self.xml.end_element();
    }

    pub fn write_region(&mut self, region: id::Region) {
        self.xml.start_element("region");
        self.xml.write_attribute("id", &region);

        for a in self.ctx.arguments(region) {
            self.xml.start_element("argument");
            self.xml.write_attribute("id", &a);
            self.xml.end_element();
        }

        for r in self.ctx.results(region) {
            self.xml.start_element("result");
            self.xml.write_attribute("id", &r);
            self.xml.end_element();
        }

        for node in self.ctx.nodes(region) {
            self.write_node(node);

            for input in self.ctx.inputs(node) {
                if let Some(origin) = self.ctx.get_input(input) {
                    self.xml.start_element("edge");
                    self.xml.write_attribute("source", &origin);
                    self.xml.write_attribute("target", &input);
                    self.xml.end_element();
                }
            }
        }

        for result in self.ctx.results(region) {
            if let Some(origin) = self.ctx.get_result(result) {
                self.xml.start_element("edge");
                self.xml.write_attribute("source", &origin);
                self.xml.write_attribute("target", &result);
                self.xml.end_element();
            }
        }

        self.xml.end_element();
    }
}

pub fn new_xml() -> XmlWriter {
    let opt = Options::default();
    let mut xml = XmlWriter::new(opt);
    xml.start_element("rvsdg");
    xml
}

pub fn open_viewer(xml: String) {
    let mut path = std::env::temp_dir();
    path.push("rvsdg.xml");
    let mut f = std::fs::File::create(&path).unwrap();
    write!(f, "{}", xml).unwrap();
    println!(" wrote to {}", path.display());

    std::process::Command::new("rvsdg-viewer")
        .arg(path)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

impl Context {
    pub fn add_to_xml(&self, xml: XmlWriter) -> XmlWriter {
        let mut ctx = XmlCtx { xml, ctx: self };
        ctx.write_node(id::AnyNode::from_u32(0));
        ctx.xml
    }

    pub fn to_xml(&self) -> String {
        let xml = new_xml();

        let mut ctx = XmlCtx { xml, ctx: self };
        ctx.write_node(id::AnyNode::from_u32(0));

        ctx.xml.end_element();

        ctx.xml.end_document()
    }
}
