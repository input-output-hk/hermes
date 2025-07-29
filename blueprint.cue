global: {
	ci: {
		local: [
			"^check(-.*)?$",
			"^build(-.*)?$",
			"^package(-.*)?$",
			"^test(-.*)?$",
		]
		registries: [
			ci.providers.aws.ecr.registry,
		]
		providers: {
			aws: {
				ecr: {
					autoCreate: true
					registry:   "332405224602.dkr.ecr.eu-central-1.amazonaws.com"
				}
				region: "eu-central-1"
				role:   "arn:aws:iam::332405224602:role/ci"
			}

			docker: credentials: {
				provider: "aws"
				path:     "global/ci/docker"
			}

			earthly: {
				satellite: credentials: {
					provider: "aws"
					path:     "global/ci/ci-tls"
				}
				version: "0.8.15"
			}

			github: registry: "ghcr.io"

			tailscale: {
				credentials: {
					provider: "aws"
					path:     "global/ci/tailscale"
				}
				tags:    "tag:cat-github"
				version: "latest"
			}
		}
		secrets: [
			{
				name:     "GITHUB_TOKEN"
				optional: true
				provider: "env"
				path:     "GITHUB_TOKEN"
			},
		]
	}
	repo: {
		defaultBranch: "main"
		name:          "input-output-hk/hermes"
	}
}
