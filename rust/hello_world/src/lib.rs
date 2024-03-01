mod player;
use godot::prelude::*;
struct HelloWorldExtension;

#[gdextension]
unsafe impl ExtensionLibrary for HelloWorldExtension {}
