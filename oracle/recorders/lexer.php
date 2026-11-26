<?php
// Records how doctrine/lexer's AbstractLexer splits inputs (its one
// preg_split() call and the tokens built from it) for the Rust port:
// crates/php2rust-doctrine-lexer/tests/data/lexer.json.
//
//   scripts/record.sh lexer
//
// Strings are hex (inputs may be invalid UTF-8). Each lexer is given its
// patterns and modifiers; "raw" lexers leave values alone and give no type,
// "concrete" is the tests' ConcreteLexer. PHP's warnings (an invalid
// pattern) are recorded, not printed.

declare(strict_types=1);

$root = dirname(__DIR__, 2) . '/upstream/doctrine/lexer';
require "$root/src/Token.php";
require "$root/src/AbstractLexer.php";
require "$root/tests/ConcreteLexer.php";

use Doctrine\Common\Lexer\AbstractLexer;
use Doctrine\Tests\Common\Lexer\ConcreteLexer;

final class RawLexer extends AbstractLexer
{
    /**
     * @param list<string> $catchable
     * @param list<string> $nonCatchable
     */
    public function __construct(
        private array $catchable,
        private array $nonCatchable,
        private string $modifiers,
    ) {
    }

    protected function getCatchablePatterns(): array
    {
        return $this->catchable;
    }

    protected function getNonCatchablePatterns(): array
    {
        return $this->nonCatchable;
    }

    protected function getModifiers(): string
    {
        return $this->modifiers;
    }

    protected function getType(string &$value): string|null
    {
        return null;
    }
}

$concrete = [['=|<|>', '[a-z]+', '\d+'], ['\s+', '(.)'], 'iu'];
$lexers = [
    'concrete' => $concrete,
    'concrete_raw' => $concrete,
    'concrete_ascii' => [$concrete[0], $concrete[1], 'i'],
    'dql' => [
        [
            '[a-z_][a-z0-9_]*\:[a-z_][a-z0-9_]*(?:\\\[a-z_][a-z0-9_]*)*',
            '[a-z_\\\][a-z0-9_]*(?:\\\[a-z_][a-z0-9_]*)*',
            '(?:[0-9]+(?:[\.][0-9]+)*)(?:e[+-]?[0-9]+)?',
            "'(?:[^']|'')*'",
            '\?[0-9]*|:[a-z_][a-z0-9_]*',
        ],
        ['\s+', '--.*', '(.)'],
        'iu',
    ],
    'annotation' => [
        [
            '[a-z_\\\][a-z0-9_\:\\\]*[a-z_][a-z0-9_]*',
            '(?:[+-]?[0-9]+(?:[\.][0-9]+)*)(?:[eE][+-]?[0-9]+)?',
            '"(?:""|[^"])*+"',
        ],
        ['[\s,]+'],
        'iu',
    ],
    'single_letter' => [['[a-z]'], ['[\s,]+'], 'iu'],
    'case_sensitive' => [['[a-z]+', '[0-9]+'], ['\s+'], 'u'],
    'empty_matches' => [['x*'], ['\s+'], 'iu'],
    'empty_and_groups' => [['a', 'b*'], ['(c)(d)?', '\s+'], 'iu'],
    'lazy' => [['a*?', 'b'], ['\s+'], 'iu'],
    'extended' => [["[a-z]+ # word\n", "\\d* # digits\n"], ['\s+'], 'iux'],
    'extended_open_comment' => [['[a-z]+ # word', '\d* # digits'], ['\s+'], 'iux'],
    'backreference' => [['(a)\2', '[b-z]+'], ['\s+'], 'iu'],
    'invalid' => [['('], ['\s+'], 'iu'],
];

$inputs = require __DIR__ . '/lexer_inputs.php';

$warnings = [];
set_error_handler(static function (int $errno, string $message) use (&$warnings): bool {
    $warnings[] = $message;

    return true;
});

function encode(mixed $value): array
{
    return match (true) {
        is_string($value) => ['s' => bin2hex($value)],
        is_int($value) => ['i' => $value],
        default => throw new LogicException(get_debug_type($value)),
    };
}

$out = [];
foreach ($lexers as $name => [$catchable, $nonCatchable, $modifiers]) {
    $lexer = $name === 'concrete' ? new ConcreteLexer() : new RawLexer($catchable, $nonCatchable, $modifiers);
    $cases = [];
    foreach ($inputs as $input) {
        $warnings = [];
        $lexer->setInput($input);
        $tokens = [];
        while (($token = $lexer->peek()) !== null) {
            $tokens[] = [encode($token->value), $token->type, $token->position];
        }
        $cases[] = ['input' => bin2hex($input), 'tokens' => $tokens, 'warnings' => $warnings];
    }
    $out[] = [
        'name' => $name,
        'catchable' => array_map('bin2hex', $catchable),
        'non_catchable' => array_map('bin2hex', $nonCatchable),
        'modifiers' => $modifiers,
        'cases' => $cases,
    ];
}

echo json_encode(['php' => PHP_VERSION, 'lexers' => $out], JSON_THROW_ON_ERROR), "\n";
