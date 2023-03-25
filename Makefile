.PHONY: upgrade-job-image
upgrade-job-image:
	@cargo build --release --bin upgrade-job
	@docker buildx build -t ${IMG_ORG}/mayastor-upgrade-job:${IMG_TAG} -f Dockerfile . --push