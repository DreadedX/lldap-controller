variable "TAG_BASE" {}
variable "RELEASE_VERSION" {}

group "default" {
  targets = ["lldap-controller", "manifests"]
}

target "docker-metadata-action" {}

target "lldap-controller" {
  inherits = ["docker-metadata-action"]
  context = "./"
  dockerfile = "Dockerfile"
  tags = [for tag in target.docker-metadata-action.tags : "${TAG_BASE}:${tag}"]
  target = "runtime"
}

target "manifests" {
  context = "./"
  dockerfile = "Dockerfile"
  target = "manifests"
  output = [{ type = "cacheonly" }, "manifests"]
}
