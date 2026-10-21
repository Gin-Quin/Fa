# Workspaces

A **workspace** is a collection of [packages](/architecture/packages) that are related to each other.

A workspace is a directory that contains a `workspace.fa` file and a `packages/` directory. The `packages/` directory is scanned to find all the folders that contain a `package.fa` file, so packages can be organized in subdirectories.

This is a typical workspace file structure:

```
myWorkspace/
├── workspace.fa
├── dependencies.fa
└── packages/
    ├── packageA/
    │   ├── package.fa
    │   ├── dependencies.fa
    │   └── library/
    │       └── foo.fa
    └── apps/
        ├── packageB/
        │   ├── package.fa
        │   ├── library/
        │   └── application/
        │       └── main.fa
        └── packageC/
            ├── package.fa
            └── application/
                └── main.fa
```

You can scaffold a workspace with the following command:

```bash
fa new workspace
```

Commands run at the root of a workspace are executed for every package that implements them. See the [overview](/architecture/overview#workspaces).
