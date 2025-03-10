# Packages

In Fa, a **package** is a collection of files that are related to each other.

There are two kinds of packages:

- **Library packages**: A library package is a package that contains a library.
- **Application packages**: An application package is a package that contains an application.

To create a **package**, you need this simple file structure:

```
myPackage/
├── dependencies.fa
├── package.fab
├── library/
│   ├── math.fa
│   └── .. your other library files ..
└── application/
    ├── main.fa
    └── .. your other application files ..
```

:::tip
If you're writing a library, you can omit the `application` folder.

If you're developing an application, you will still need the `library` folder. It's going to be your "local" library.
:::

You can scaffold a package with the following command:

```bash
fa new package
```

## Package configuration

