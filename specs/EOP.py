# Entity-oriented-programming

# I discovered ECS (Entity-Component-System) model not so long ago
# and I see a lot of great things in this model - especially for
# creative programming

# It has the following awesome features :
# - Composition system over inheritance, very much more flexible
# - Dynamic inheritance
# - Offers the capabilities of hot-reloading while programming (even with AOT compilation)

# The ECS is especially suited to games
# But I see links between games and documents and I would like to try merge all this
# into one single language made especially for *creative programming*


## EXPLANATION TIME

# I really want Fa's core to work this way :
# - the logic and structures are described in .fa files
# - the data itself (completely logicless) is stored into .do files (equivalent of the html / the json)
# - (perhaps .mi files could be the binary version of .do files -- see message pack)
# - there is a main .fx file that is compilable / runnable

# Example of what I would like to have :
zabu: SearchCrumb
	@Style
		width 321
		height 321
		font 321
		fontZoom 2.4
	@Body
		weight 134
		speed 12

# It makes things very clean because all properties depend on a component
# Ideally, we should be able to add scripts too -- to be thinked


# Let's take a html file :
 <div class="search-crumb">
     <div @click="gotoCrumb(key)" v-for="(criteria,key) in criteriasLabel" :key="key">
         <span class="title">{{$t(stagesDatas[key].title_key)}}</span>
         <span>{{ key === 0 ? currentSupplierName : (stagesDatas[key].title_key === 'customer_code' && criteria ? "******" : criteria ) }}</span>
     </div>
     <div class="current">
         {{current ? $t(current) : $t(stages[currentStage])}}
     </div>
 </div>

# Now let's suppose we want to write it with my entity syntax :
- SearchCrumb
	- Element for criteria in criteriasLabel
		@Style
			direction vertical
		- Title stagesDatas[key].title_key
		- Text criteria
	- Text current

# Explanation :
# - The entity template `SearchCrumb` has been created before with default components & values, like so :

entity SearchCrumb
	state = 13
	zabu = 3521

	@Style
		...
	@Events
		...

# - Of course, every component can be dynamically added/deleted (thats one core principle of ECS)
# That's actually the difference between an entity and a class : for a class, components are *static*