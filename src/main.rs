use three_d::*;

// Struktura reprezentująca punkt masy
#[derive(Clone, Debug)]
struct MassPoint {
    position: Vec3,
    velocity: Vec3,
    mass: f32,
    fixed: bool, // Czy punkt jest nieruchomy (np. podstawa wędki)
}

impl MassPoint {
    fn new(position: Vec3, mass: f32, fixed: bool) -> Self {
        Self {
            position,
            velocity: Vec3::zero(),
            mass,
            fixed,
        }
    }
}

// Struktura reprezentująca sprężynę
#[derive(Clone, Debug)]
struct Spring {
    point_a: usize,
    point_b: usize,
    rest_length: f32,
    stiffness: f32,
    damping: f32,
}

impl Spring {
    fn new(point_a: usize, point_b: usize, rest_length: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            point_a,
            point_b,
            rest_length,
            stiffness,
            damping,
        }
    }
}

// System fizyki mass-spring
struct MassSpringSystem {
    masses: Vec<MassPoint>,
    springs: Vec<Spring>,
    gravity: Vec3,
}

impl MassSpringSystem {
    fn new(gravity: Vec3) -> Self {
        Self {
            masses: Vec::new(),
            springs: Vec::new(),
            gravity,
        }
    }

    fn add_mass(&mut self, mass: MassPoint) -> usize {
        self.masses.push(mass);
        self.masses.len() - 1
    }

    fn add_spring(&mut self, spring: Spring) {
        self.springs.push(spring);
    }

    fn update(&mut self, dt: f32) {
        // Oblicz siły sprężyn
        let mut forces: Vec<Vec3> = vec![Vec3::zero(); self.masses.len()];

        for spring in &self.springs {
            let pos_a = self.masses[spring.point_a].position;
            let pos_b = self.masses[spring.point_b].position;
            let vel_a = self.masses[spring.point_a].velocity;
            let vel_b = self.masses[spring.point_b].velocity;

            let delta = pos_b - pos_a;
            let distance = delta.magnitude();

            if distance > 0.0001 {
                let direction = delta / distance;

                // Siła sprężyny (prawo Hooke'a)
                let spring_force = direction * spring.stiffness * (distance - spring.rest_length);

                // Siła tłumienia
                let relative_velocity = vel_b - vel_a;
                let damping_force = direction * spring.damping * relative_velocity.dot(direction);

                let total_force = spring_force + damping_force;

                forces[spring.point_a] += total_force;
                forces[spring.point_b] -= total_force;
            }
        }

        // Zastosuj siły i zaktualizuj pozycje
        for (i, mass) in self.masses.iter_mut().enumerate() {
            if !mass.fixed {
                // Dodaj grawitację
                let total_force = forces[i] + self.gravity * mass.mass;

                // F = ma -> a = F/m
                let acceleration = total_force / mass.mass;

                // Integracja Eulera
                mass.velocity += acceleration * dt;
                mass.position += mass.velocity * dt;
            }
        }
    }
}

// Model wędki
struct FishingRod {
    system: MassSpringSystem,
    segment_count: usize,
}

impl FishingRod {
    fn new(base_position: Vec3, length: f32, segment_count: usize) -> Self {
        let mut system = MassSpringSystem::new(vec3(0.0, -9.81, 0.0));

        let segment_length = length / segment_count as f32;
        let mass_per_segment = 0.1;
        let stiffness = 500.0;
        let damping = 5.0;

        // Twórz punkty masy wzdłuż wędki
        let mut previous_index = None;
        for i in 0..=segment_count {
            let t = i as f32 / segment_count as f32;
            let position = base_position + vec3(0.0, length * t, 0.0);
            let is_fixed = i == 0; // Pierwszy punkt jest nieruchomy (podstawa)

            let index = system.add_mass(MassPoint::new(position, mass_per_segment, is_fixed));

            // Połącz z poprzednim punktem sprężyną
            if let Some(prev_idx) = previous_index {
                system.add_spring(Spring::new(
                    prev_idx,
                    index,
                    segment_length,
                    stiffness,
                    damping,
                ));
            }

            previous_index = Some(index);
        }

        Self {
            system,
            segment_count,
        }
    }

    fn update(&mut self, dt: f32) {
        self.system.update(dt);
    }

    fn get_positions(&self) -> Vec<Vec3> {
        self.system.masses.iter().map(|m| m.position).collect()
    }

    // Dodaj siłę zewnętrzną do końcówki wędki (np. od ryby lub wiatru)
    fn apply_force_to_tip(&mut self, force: Vec3) {
        if let Some(last) = self.system.masses.last_mut() {
            if !last.fixed {
                last.velocity += force / last.mass * 0.016; // Zakładając dt=0.016
            }
        }
    }
}

fn main() {
    // Twórz okno i kontekst
    let window = Window::new(WindowSettings {
        title: "3D Fishing Rod Simulation - Mass-Spring Model".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();

    let context = window.gl();

    // Utwórz kamerę
    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(5.0, 3.0, 5.0),
        vec3(0.0, 2.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        0.1,
        1000.0,
    );

    let mut control = OrbitControl::new(*camera.target(), 1.0, 100.0);

    // Twórz światło
    let light = DirectionalLight::new(&context, 2.0, Srgba::WHITE, &vec3(-1.0, -1.0, -1.0));

    // Twórz planszę (podłogę)
    let mut board_cpu_mesh = CpuMesh::square();
    board_cpu_mesh.transform(&Mat4::from_scale(10.0)).unwrap();
    board_cpu_mesh
        .transform(&Mat4::from_angle_x(degrees(-90.0)))
        .unwrap();

    let mut board = Gm::new(
        Mesh::new(&context, &board_cpu_mesh),
        PhysicalMaterial::new_opaque(
            &context,
            &CpuMaterial {
                albedo: Srgba::new(100, 150, 100, 255),
                ..Default::default()
            },
        ),
    );

    // Twórz siatkę na planszy
    let grid_size = 10;
    let grid_spacing = 1.0;
    let mut grid_lines = Vec::new();

    for i in 0..=grid_size {
        let offset = (i as f32 - grid_size as f32 / 2.0) * grid_spacing;
        // Linie wzdłuż osi X
        grid_lines.push(vec3(-grid_size as f32 / 2.0 * grid_spacing, 0.01, offset));
        grid_lines.push(vec3(grid_size as f32 / 2.0 * grid_spacing, 0.01, offset));
        // Linie wzdłuż osi Z
        grid_lines.push(vec3(offset, 0.01, -grid_size as f32 / 2.0 * grid_spacing));
        grid_lines.push(vec3(offset, 0.01, grid_size as f32 / 2.0 * grid_spacing));
    }

    let grid = Gm::new(
        Mesh::new(
            &context,
            &CpuMesh {
                positions: Positions::F32(grid_lines),
                ..Default::default()
            },
        ),
        ColorMaterial {
            color: Srgba::new(50, 50, 50, 255),
            ..Default::default()
        },
    );

    // Twórz wędkę
    let mut fishing_rod = FishingRod::new(vec3(0.0, 0.0, 0.0), 3.0, 15);

    // Zmienna do symulacji czasu
    let mut time = 0.0f32;

    // Pętla renderowania
    window.render_loop(move |mut frame_input| {
        let dt = 0.016; // ~60 FPS
        time += dt;

        // Zastosuj siłę do końcówki wędki (symulacja wiatru lub ruchu)
        let wind_force = vec3(
            (time * 2.0).sin() * 2.0,
            0.0,
            (time * 3.0).cos() * 1.5,
        );
        fishing_rod.apply_force_to_tip(wind_force);

        // Aktualizuj symulację wędki
        fishing_rod.update(dt);

        // Aktualizuj kontrolę kamery
        control.handle_events(&mut camera, &mut frame_input.events);

        // Twórz geometrię dla wędki
        let rod_positions = fishing_rod.get_positions();
        let mut rod_vertices = Vec::new();

        for i in 0..rod_positions.len() - 1 {
            rod_vertices.push(rod_positions[i]);
            rod_vertices.push(rod_positions[i + 1]);
        }

        let rod_mesh = Gm::new(
            Mesh::new(
                &context,
                &CpuMesh {
                    positions: Positions::F32(rod_vertices),
                    ..Default::default()
                },
            ),
            ColorMaterial {
                color: Srgba::new(139, 69, 19, 255), // Brązowy kolor
                ..Default::default()
            },
        );

        // Twórz kule w punktach masy
        let mut spheres = Vec::new();
        for (i, pos) in rod_positions.iter().enumerate() {
            let radius = if i == 0 { 0.08 } else { 0.05 }; // Większa kula u podstawy
            let mut sphere = Gm::new(
                Mesh::new(&context, &CpuMesh::sphere(8)),
                PhysicalMaterial::new_opaque(
                    &context,
                    &CpuMaterial {
                        albedo: if i == 0 {
                            Srgba::new(200, 50, 50, 255)
                        } else {
                            Srgba::new(139, 69, 19, 255)
                        },
                        ..Default::default()
                    },
                ),
            );
            sphere.set_transformation(
                Mat4::from_translation(*pos) * Mat4::from_scale(radius)
            );
            spheres.push(sphere);
        }

        // Renderuj scenę
        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.5, 0.7, 1.0, 1.0, 1.0))
            .render(&camera, &board, &[&light])
            .render(&camera, &grid, &[&light])
            .render(&camera, &rod_mesh, &[&light])
            .render(&camera, spheres.iter().collect::<Vec<_>>().as_slice(), &[&light]);

        FrameOutput::default()
    });
}
