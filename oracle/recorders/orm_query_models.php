<?php
// Describes the models of the doctrine/orm DQL tests (the Parser, SqlWalker
// and tree walker tests) for crates/php2rust-doctrine-orm/tests/common/
// models.rs, through orm_models.php: the test files, the models that their
// DQL strings and imports name, and the whole CMS and Company model sets.

$root  = __DIR__ . '/../../upstream/doctrine/orm/tests/Tests';
$files = [
    'ORM/Query/LanguageRecognitionTest.php',
    'ORM/Query/SelectSqlGenerationTest.php',
    'ORM/Query/DeleteSqlGenerationTest.php',
    'ORM/Query/UpdateSqlGenerationTest.php',
    'ORM/Query/ParserTest.php',
    'ORM/Query/ParserResultTest.php',
    'ORM/Query/CustomTreeWalkersTest.php',
    'ORM/Query/CustomTreeWalkersJoinTest.php',
    'ORM/Query/SqlWalkerTest.php',
    'ORM/Query/TypedExpressionDeprecationTest.php',
];

$classes = [];
foreach ($files as $file) {
    $source = str_replace('\\\\', '\\', file_get_contents("$root/$file"));
    preg_match_all('/Doctrine\\\\Tests\\\\Models\\\\[A-Za-z0-9_]+\\\\[A-Z][A-Za-z0-9_]*/', $source, $matches);
    foreach ($matches[0] as $class) {
        $classes[$class] = true;
    }
}

foreach (['CMS', 'Company'] as $set) {
    foreach (glob("$root/Models/$set/*.php") as $path) {
        $classes["Doctrine\\Tests\\Models\\$set\\" . basename($path, '.php')] = true;
    }
}

ksort($classes);
$argv = [__FILE__, ...$files, ...array_keys($classes)];
require __DIR__ . '/orm_models.php';
