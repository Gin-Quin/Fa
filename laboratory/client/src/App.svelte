<script>
	import {
		MaterialApp,
		NavigationDrawer,
		Card,
		CardText,
		ProgressLinear,
		Select,
		Subheader,
	} from 'svelte-materialify'

	import Code from './sections/Code.svelte'
	import Json from './sections/Json.svelte'
	
	let theme = 'light'
	let targets = [
		{ name: 'Javascript', value: 'Javascript'},
		{ name: 'C++', value: 'C++'},
		{ name: 'Typescript', value: 'Typescript'},
	]
	let target = targets[0].name
	let stages = [
		{
			name: "Fa",
			component: Code,
			operation: "Parse",
			visible: true,
			mode: 'nim',
		},
		{
			name: "Tokens",
			component: Json,
			operation: "Transform",
			visible: true,
			toggable: true,
			json: {x: 12, y: 125, zabu: true, mom: "Jeanne", friends: [{name: 'Coco', strength: 12}, {name: 'Zabu', strength: 11}, 'Hercule', 'Ajax', "Mamie Molette"], friends2: ['Coco', 'Zabu', 'Hercule', 'Ajax', "Mamie Molette"]},
		},
		{
			name: "Raw Ast",
			component: Json,
			operation: "Validate",
			visible: false,
			toggable: true,
			json: {x: 1287687},
		},
		{
			name: "Ast",
			component: Json,
			operation: "Generate", 
			visible: true,
			toggable: true,
			json: {x: 12, y: {z: 56, t: [3, 5, 6, {x: 12, y: {z: 56, t: {x: 12, y: {z: 56, t: [3, 5, 6, {x: 12, y: {z: 56, t: {x: 12, y: {z: 56, t: [3, 5, 6, {x: 12, y: {z: 56, t: [3, 5, 6]}}]}}}}]}}}}]}},
		},
		{
			name: target,
			component: Code,
			visible: true,
			toggable: true,
			readonly: true,
			mode: target.toLowerCase(),
		},
	]

	$: stages[stages.length - 1].name = target


</script>


<MaterialApp {theme}>

	<div class="app d-flex">

		<NavigationDrawer>

			<header class="">
				<div class="title primary-text">
					<img src="favicon.ico" alt="fa-laboratory" width="42px">
					<span>
						Fa • Laboratory
					</span>
				</div>

				<Select
					items={targets}
					bind:value={target}
					mandatory
					outlined
					dense
				>
					Target
				</Select>
			</header>

			<div class="library">
				<Subheader>Library</Subheader>
					
			</div>

		</NavigationDrawer>
	
		<main>
			<header class="timeline">
				<div class="stages">
					{#each stages as stage}
						<div
							class:clickable={stage.toggable}
							class:stage={true}
							class:primary-text={stage.visible}
							on:click={() => stage.visible = (stage.toggable ? !stage.visible : stage.visible)}
						>
							<Card raised={stage.visible}>
								<CardText>
									{stage.name}
								</CardText>
							</Card>
						</div>
			
						{#if stage.operation}
							<div class="operation">
							<span class="name primary-text">
									{stage.operation}
								</span>
								<ProgressLinear />
								<span class="time">
									{#if stages.operationTime}
										{stage.operationTime + ' ms'}
									{:else}
										<br>
									{/if}
								</span>
								</div>
						{/if}
					{/each}
				</div>
		
			</header>
			
			<div class="sections d-flex">
				{#each stages as stage}
					{#if stage.visible}
						<div class="section">
							<div class="title" class:light={stage.component.name == 'Json'} >{stage.name}</div>
							<svelte:component this={stage.component} {stage} />
						</div>
					{/if}
				{/each}
			</div>
			
		</main>

	</div>


		
</MaterialApp>



<style lang="sass">
	:global(body)
		padding: 0
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif
	
	:global(.s-app)
		width: 100vw
		height: 100vh
		display: flex
		flex-direction: column
	
	:global(input)
		margin: 0 !important
	
	:global(.s-card)
		min-width: 97px
		text-align: center
	
	.app
		display: flex
		height: 100%
	
	header
		display: flex
		flex-direction: column
		justify-content: space-evenly
		
		>.title
			display: flex
			align-items: center
			justify-content: center
			flex: 1

			> span
				font-weight: bold		

	.timeline
		height: 130px
		background: #f5f5f5
		padding-bottom: 22px


	.stages
		display: flex
		align-items: center
		flex: 1 0
		justify-content: center

	.stage
		cursor: default
	
	.clickable
		cursor: pointer
	
	.operation
		color: #ccc
		text-align: center
		width: 92px
		font-size: 13px
		font-style: italic

		> .name
			opacity: 0.5

	main
		display: flex
		flex-direction: column
		flex: 1

	.sections
		flex: 1
		flex-wrap: wrap
	
	.section
		flex: 1 0
		min-width: 320px
		border-top: 1px solid #ddd
		// height: 100%
		position: relative
		text-align: center
		display: flex
		justify-content: center

		&:not(:last-child)
			border-right: 1px solid #ddd
		
		> .title
			position: absolute
			bottom: 100%
			font-size: 11px
			padding: 6px 12px 0
			background: rgb(36, 36, 36)
			color: white
			border: 1px solid #ddd
			border-top-left-radius: 15px
			border-top-right-radius: 15px
			z-index: 5
			line-height: 12px
			font-weight: bold
			// box-shadow: 0 -2px 6px -2px rgb(138 138 138 / 50%)

			&.light
				background: rgb(242, 242, 242)
				color: #555

</style>
