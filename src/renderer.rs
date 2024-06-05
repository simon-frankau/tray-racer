//
// Display-independent rendering of the scene.
//

pub fn file_texture() -> Vec<u8> {
    // TODO: Select the images we want and put them into source control.
    image::open("skyboxes/night-skyboxes/NightPath/negx.jpg")
        .unwrap()
        .into_rgba8()
        .to_vec()
}
