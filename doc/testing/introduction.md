# testing

This section describes all libraries and tools in jormungandr repo which purpose is to satifsy quality needs of jormungandr and jcli components

Jormungandr test libraries includes projects:

* jormungandr-automation - sets of apis for automating all node calls and node sub-components (REST, GRPC, logging etc.),
* hersir - api & cli for bootstrapping entire network of nodes with some predefined configuration. Project takes care of proper settings for all nodes as well as block0,
* thor - testing api & cli for all wallet operations,
* mjolnir - load tool (api & cli) for all kind of jormungandr transactions,
* loki - api & cli for sending invalid/adversary load as well as boostraping adversary node.
