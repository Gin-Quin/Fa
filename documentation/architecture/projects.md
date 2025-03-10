# Workspaces

A **workspace** is a collection of projects that are related to each other.

The file structure of a workspace is slightly different from a regular project. Instead of having a `project.fab` file, a `library` and `application` folder, you'll have a `workspace.fab` file and a `projects` folder.

This is a typical workspace file structure:

```
myWorkspace/
├── workspace.fab
├── dependencies.fa
├── projects/
│   ├── projectA/
│   │   ├── project.fab
│   │   ├── dependencies.fa
│   │   ├── library/
│   ├── projectB/
│   │   ├── project.fab
│   │   ├── dependencies.fa
│   │   ├── library/
│   │   └── application/
└── └── ...
```

You can scaffold a project with the following command:

```bash
fa new project
```
