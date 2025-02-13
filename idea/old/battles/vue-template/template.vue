<template>
    <div class="module-fields-table">

        <!-- table header -->
        <div class="chemical-header">
            <div class="module">
                <span>{{$t("module_souscrit")}}</span>
            </div>
            <div v-for="key in columns" :class="key">
                <w-checkbox v-if="key=='selected'" circled bigger black v-model="selectAll" />
                <span v-else>{{$t(key)}}</span>
            </div>
        </div>

        <!-- table content (module by module) -->
        <div
            v-for="module, moduleName in value"
            class="chemical-module"
        >
            <div class="module" @click="clickOnModule(module)">
                <span>{{$t(moduleName)}}</span>
            </div>

            <div class="module-data">
                <div
                    v-for="data in module"
                    class="chemical-module-row"
                    :selected="data.selected"
                    @click="data.selected = !data.selected"
                >
                    <div v-for="key in columns" :class="key">

                        <w-checkbox
                            v-if="key=='selected'"
                            circled
                            bigger
                            :value="data.selected"

                        />

                        <span v-else>
                            {{$t(data[key])}}
                        </span>
                    </div>
                </div>
            </div>
        </div>

    </div>
</template>


<script src="./logic.ts" lang="ts"></script>
<style src="./skin.styl" lang="stylus" scoped></style>
