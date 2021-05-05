# Settings interfaces for the jormungandr node

Exposes only the APIs of how to update or get the settings of the node.
The content is very much opinionated already. We expects the keys and the
value to be `String`.

Underlying, it uses `sled`. Though, except for the constructor itself
the API hides entirely what's going on in the backend.
