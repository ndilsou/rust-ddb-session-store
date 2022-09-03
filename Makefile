build:
	cargo lambda build --release

watch:
	cargo lambda watch

deploy:
	yarn cdk deploy