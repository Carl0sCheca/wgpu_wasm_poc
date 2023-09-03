#[derive(Debug)]
pub struct Transform {
    pub label: Option<String>,
    pub position: nalgebra_glm::Vec3,
    pub rotation: nalgebra_glm::Vec3,
    pub scale: nalgebra_glm::Vec3,
    pub matrix: nalgebra_glm::Mat4,
    pub index: i32,
    pub flip_x: i32,
    parent: Option<std::rc::Weak<std::cell::RefCell<Transform>>>,
    children: Vec<std::rc::Rc<std::cell::RefCell<Transform>>>,
}

type TransformRc = std::rc::Rc<std::cell::RefCell<Transform>>;

#[allow(dead_code)]
impl Transform {
    pub fn new() -> Self {
        Self {
            position: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            rotation: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            scale: nalgebra_glm::vec3(1.0, 1.0, 1.0),
            matrix: nalgebra_glm::Mat4::identity(),
            index: 0,
            flip_x: 0,
            parent: None,
            children: vec![],
            label: None,
        }
    }

    /// This function only works when you parent an object without scale and rotation.
    /// TODO: fix this any time
    pub fn parent(parent: TransformRc, child: TransformRc) {
        parent.as_ref().borrow_mut().children.push(child.clone());
        child.as_ref().borrow_mut().parent = Some(std::rc::Rc::downgrade(&parent.clone()));

        let child_position = child.as_ref().borrow().position.clone();
        let parent_position = parent.as_ref().borrow().global_position().clone();
        let child_scale = child.as_ref().borrow().scale.clone();
        let parent_scale = parent.as_ref().borrow().scale.clone();
        let child_rotation = child.as_ref().borrow().rotation.clone();
        let parent_rotation = parent.as_ref().borrow().global_rotation().clone();

        let new_position = child_position - parent_position;
        let new_rotation = child_rotation - parent_rotation;

        child
            .as_ref()
            .borrow_mut()
            .scale(&child_scale.component_div(&parent_scale));
        child.as_ref().borrow_mut().rotate(&new_rotation);
        child.as_ref().borrow_mut().translate(&new_position);
    }

    pub fn global_position(&self) -> nalgebra_glm::Vec3 {
        if let Some(parent_cell) = &self.parent {
            if let Some(parent) = parent_cell.clone().upgrade() {
                let parent = unsafe { &(*parent.as_ptr()) };
                parent.global_position() + self.position
            } else {
                self.position
            }
        } else {
            self.position
        }
    }

    pub fn global_rotation(&self) -> nalgebra_glm::Vec3 {
        if let Some(parent_cell) = &self.parent {
            if let Some(parent) = parent_cell.clone().upgrade() {
                let parent = unsafe { &(*parent.as_ptr()) };
                parent.global_rotation() + self.rotation
            } else {
                self.rotation
            }
        } else {
            self.rotation
        }
    }

    pub fn to_raw(&self) -> TransformRaw {
        TransformRaw {
            transform: self.matrix.into(),
            index: self.index,
            flip_x: self.flip_x,
        }
    }

    pub fn forward(&self) -> nalgebra_glm::Vec3 {
        let rotation_y = nalgebra_glm::rotate_y(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.y.to_radians(),
        );
        let rotation_x = nalgebra_glm::rotate_x(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.x.to_radians(),
        );
        let rotation_z = nalgebra_glm::rotate_z(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.z.to_radians(),
        );

        // Y * X * Z
        let rotation_matrix = (rotation_y * rotation_x * rotation_z).normalize();

        nalgebra_glm::column(&rotation_matrix, 2).xyz()
    }

    pub fn up(&self) -> nalgebra_glm::Vec3 {
        let rotation_y = nalgebra_glm::rotate_y(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.y.to_radians(),
        );
        let rotation_x = nalgebra_glm::rotate_x(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.x.to_radians(),
        );
        let rotation_z = nalgebra_glm::rotate_z(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.z.to_radians(),
        );

        // Y * X * Z
        let rotation_matrix = (rotation_y * rotation_x * rotation_z).normalize();

        nalgebra_glm::column(&rotation_matrix, 1).xyz()
    }

    pub fn left(&self) -> nalgebra_glm::Vec3 {
        let rotation_y = nalgebra_glm::rotate_y(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.y.to_radians(),
        );
        let rotation_x = nalgebra_glm::rotate_x(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.x.to_radians(),
        );
        let rotation_z = nalgebra_glm::rotate_z(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.z.to_radians(),
        );

        // Y * X * Z
        let rotation_matrix = (rotation_y * rotation_x * rotation_z).normalize();

        nalgebra_glm::column(&rotation_matrix, 0).xyz()
    }

    pub fn get_local_model_matrix(&self) -> nalgebra_glm::Mat4 {
        let rotation_y = nalgebra_glm::rotate_y(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.y.to_radians(),
        );
        let rotation_x = nalgebra_glm::rotate_x(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.x.to_radians(),
        );
        let rotation_z = nalgebra_glm::rotate_z(
            &nalgebra_glm::Mat4::identity(),
            self.rotation.z.to_radians(),
        );

        // Y * X * Z
        let rotation_matrix = rotation_y * rotation_x * rotation_z;

        // translation * rotation * scale (also known as TRS matrix)
        nalgebra_glm::translate(&nalgebra_glm::Mat4::identity(), &self.position)
            * rotation_matrix
            * nalgebra_glm::scale(&nalgebra_glm::Mat4::identity(), &self.scale)
    }

    pub fn update_self_and_child(&mut self) {
        if let Some(parent_cell) = &mut self.parent {
            if let Some(parent) = parent_cell.clone().upgrade() {
                let parent_matrix = unsafe { (*parent.as_ptr()).matrix };
                self.matrix = parent_matrix * self.get_local_model_matrix();
            }
        } else {
            self.matrix = self.get_local_model_matrix();
        }

        for child in &self.children {
            child.as_ref().borrow_mut().update_self_and_child();
        }
    }

    pub fn translate(&mut self, position: &nalgebra_glm::Vec3) {
        // self.matrix = self.matrix * nalgebra_glm::translation(position);
        self.position = *position;
        self.update_self_and_child();
    }

    pub fn rotate(&mut self, rotation: &nalgebra_glm::Vec3) {
        // let rotation_y =
        //     nalgebra_glm::rotate_y(&nalgebra_glm::Mat4::identity(), rotation.y.to_radians());
        // let rotation_x =
        //     nalgebra_glm::rotate_x(&nalgebra_glm::Mat4::identity(), rotation.x.to_radians());
        // let rotation_z =
        //     nalgebra_glm::rotate_z(&nalgebra_glm::Mat4::identity(), rotation.z.to_radians());

        // self.matrix = self.matrix * rotation_z * rotation_y * rotation_x;
        self.rotation = *rotation;

        self.update_self_and_child();
    }

    pub fn scale(&mut self, scale: &nalgebra_glm::Vec3) {
        self.scale = *scale;
        self.update_self_and_child();
        // self.matrix = self.matrix * nalgebra_glm::scaling(&scale);
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformRaw {
    pub transform: [[f32; 4]; 4],
    pub index: i32,
    pub flip_x: i32,
}
