<?php
// Records the tokens doctrine/orm's DQL lexer (Doctrine\ORM\Query\Lexer)
// makes for the Rust port:
// crates/php2rust-doctrine-orm/tests/data/query_lexer.json.
//
//   scripts/record.sh orm-query-lexer
//
// The inputs are lexer.php's, then every keyword and token name in three
// cases, edge cases of getType(), and 400 strings drawn from DQL words and
// symbols. Strings are hex; a type is its TokenType case name.

declare(strict_types=1);

$root = dirname(__DIR__, 2) . '/upstream/doctrine';
require "$root/lexer/src/Token.php";
require "$root/lexer/src/AbstractLexer.php";
require "$root/orm/src/Query/TokenType.php";
require "$root/orm/src/Query/Lexer.php";

use Doctrine\ORM\Query\Lexer;
use Doctrine\ORM\Query\TokenType;

$inputs = require __DIR__ . '/lexer_inputs.php';

foreach (TokenType::cases() as $case) {
    $word = substr($case->name, 2);
    array_push($inputs, $word, strtolower($word), ucfirst(strtolower($word)), $case->name);
}
array_push(
    $inputs,
    "'", "''", "'a''", "'a''b'", "''''", "'it''s' 'x'", 'a:b', 'a:b:c', 'a:b\\c\\d', '\\Foo\\Bar',
    'Foo\\', 'Foo\\\\Bar', '\\', '?', '?12', '?a', ':', ':1', ':_x', ':x:y', "--comment\nSELECT",
    '-- only', '{a}', '!', '/', '*', '+', '-', '%', '&', '1e', '1E5', '1e+5', '1e-5', '.5', '5.',
    '1.2.3', '00012', '0x1A', 'e5', '_', '__', "\xC4\xB1n", "\xC4\xB0N", "\xC5\xBFelect", "s\xC5\xBF",
    "\xE2\x84\xAAEY", 'SELECT u.id, COUNT(u) AS HIDDEN c FROM User u GROUP BY u.id HAVING c > 1',
    'UPDATE A a SET a.b = NULL WHERE a.c IN (1, 2) OR NOT a.d = TRUE',
    "SELECT NEW App\\Dto(u.id) FROM App:User u WHERE u.n LIKE '%x%' ESCAPE '!'",
);

mt_srand(42);
$words = [
    'SELECT', 'select', 'u', 'u.id', 'FROM', 'App\\User', 'a:b', 'WHERE', 'and', 'Or', 'not',
    'IN', 'LIKE', 'null', 'TRUE', 'false', 'identifier', 'fully_qualified_name', 'comma', 'none',
    'x_1', '_y', '1', '1.5', '2e3', "'s'", "'it''s'", '?1', '?', ':p', '(', ')', ',', '.', '=',
    '<>', '<=', '>', '+', '-', '*', '/', '!', '{', '}', '--c', ' ', '  ', "\n", "\t", "\xC3\xA9",
];
for ($i = 0; $i < 400; $i++) {
    $count = mt_rand(1, 10);
    $input = '';
    for ($j = 0; $j < $count; $j++) {
        $input .= $words[mt_rand(0, count($words) - 1)];
        if (mt_rand(0, 2) > 0) {
            $input .= ' ';
        }
    }
    $inputs[] = $input;
}

$cases = [];
foreach ($inputs as $input) {
    $lexer = new Lexer($input);
    $tokens = [];
    while (($token = $lexer->peek()) !== null) {
        $tokens[] = [bin2hex($token->value), $token->type->name, $token->position];
    }
    $cases[] = ['input' => bin2hex($input), 'tokens' => $tokens];
}

echo json_encode(['php' => PHP_VERSION, 'cases' => $cases], JSON_THROW_ON_ERROR), "\n";
