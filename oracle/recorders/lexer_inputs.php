<?php
// The inputs of oracle/recorders/lexer.php (and the start of
// orm_query_lexer.php's): fixed strings, then 400 drawn with mt_rand().

declare(strict_types=1);

$fixed = [
    '', ' ', 'price=10', 'price = 10', 'a b  c', "\n\t x \r\n", 'one', 'ONE two',
    "\xE9=10", "caf\xC3\xA9 = 12", "h\xC3\xA9llo w\xC3\xB6rld 12", "\xE2\x84\xAA", "\xC5\xBF",
    "\xC4\xB0\xC4\xB1", "\xE6\x97\xA5\xE6\x9C\xAC 123", '1.5e3', '1.2.3', 'a,b,,c', 'x', 'xx a xxx',
    'abcd', 'cd c', 'bbb ccc d',
    "SELECT u FROM App\\Entity\\User u WHERE u.id = :id AND u.name = 'x''y' -- end",
    'SELECT a:b FROM c WHERE d = ?1 OR e = ?', '@ODM\\Id', '@Foo("bar", 1.5e-3)', "'open",
    "\x00", "a\x00b", "\xFF\xFE",
];

mt_srand(20261126);
$alphabet = [
    'a', 'b', 'c', 'd', 'Z', 'x', 'y', 'e', '1', '0', '9', '.', '=', '<', '>', ' ', "\t", "\n", ',',
    ':', '?', "'", '"', '\\', '-', '_', '(', ')', '@', '+', "\xC3\xA9", "\xC4\xB0", "\xC4\xB1",
    "\xE2\x84\xAA", "\xC5\xBF", "\xE6\x97\xA5", "\xE9", "\xC3",
];
$inputs = $fixed;
for ($i = 0; $i < 400; $i++) {
    $length = mt_rand(0, 14);
    $input = '';
    for ($j = 0; $j < $length; $j++) {
        $input .= $alphabet[mt_rand(0, count($alphabet) - 1)];
    }
    $inputs[] = $input;
}

return $inputs;
