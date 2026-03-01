# mdingress

A k8s component which creates mDNS hosts for ingresses.

## Overview

mdingress watches for ingresses with `host: something.local` set, and integrates with whatever node it's running on to publish services.

## Requirements

- The node should have `avahi-daemon` running, with a socket located at `/run/avahi-daemon/socket` and dbus enabled. It will probably be setup this way by default on your distro.
- The ingress can contain one or more paths, but only the first path with `host: *.local` will be used.

## Setup

Clone the repo and run `just build` (or `just publish` if you have a registry you can directly push to). This will build an image compatible with `amd64` and `arm64` architectures.

An example deployment is provided in `manifests/`. It creates a namespace, deployment and the RBAC resources required to watch for ingress events.

If you apply that and check the logs, you should see it picking up applicable Ingresses and publishing with avahi. When you remove an ingress, mdingress will shutdown the corresponding service.

A quick way to check that things are working is to just `ping something.local`! The hostname will resolve to the IP address mdingress' node.

> Because ingresses listen on every node, it shouldn't matter which one mdingress itself is running on. All that matters is your requests reach one of the nodes. The ingress controller (at least Traefik in my case) will see the destination `something.local` in the request header and route it appropriately.