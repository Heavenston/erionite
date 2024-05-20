# Reasoning for various design and technical choices

## `allow(clippy::type_complexity)` on bevy systems
This is bevy queries -> type complexity is expected and i do not think 
adding indirection will make it clearer, it will even make it less clear

## `allow(clippy::too_many_arguments)` on bevy systems
Same as for type complexity, although it should generally still be avoided
