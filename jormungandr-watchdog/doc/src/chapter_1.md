# Introduction

There are different modules or services in Jörmungandr's node. These components
are responsible for different parts of the activities involved running the
protocol. We currently have a pretty much static model to run them. It is
working well enough to run the basic features of the protocol. However it is
very much limited and as we add more and more features it will be more
difficult to integrate new features.

This document presents a generic purpose library to maintain services in a
single process and connect them together. This document will present the
different requirements, projected to Jörmungandr's use case.
