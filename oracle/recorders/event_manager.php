<?php
// Records doctrine/event-manager's ordering and removal rules for the Rust
// port: crates/php2rust-doctrine-event-manager/tests/data/event_manager.json.
//
//   scripts/record.sh event-manager
//
// One scenario: listeners x, y, z added, re-added, removed and added again;
// after each dispatch, the methods called in order; the events listed by
// getAllListeners() once an event lost its last listener; whether
// dispatching such an event creates the empty EventArgs; what calling a
// missing listener method throws.

declare(strict_types=1);

$src = dirname(__DIR__, 2) . '/upstream/doctrine/event-manager/src/';
foreach ([
    'EventArgs', 'EventDispatcher', 'EventListenerIntrospector', 'EventListenerRegistry',
    'EventSubscriberRegistry', 'EventManagerInterface', 'EventSubscriber', 'EventManager',
] as $file) {
    require "$src$file.php";
}

use Doctrine\Common\EventArgs;
use Doctrine\Common\EventManager;

final class Listener
{
    /** @param list<string> $log */
    public function __construct(public string $name, public array &$log)
    {
    }

    public function a(EventArgs $e): void
    {
        $this->log[] = $this->name . ':a';
    }

    public function b(EventArgs $e): void
    {
        $this->log[] = $this->name . ':b';
    }
}

$log = [];
$out = [];
$em = new EventManager();
$x = new Listener('x', $log);
$y = new Listener('y', $log);
$z = new Listener('z', $log);

$em->addEventListener(['a'], $x);
$em->addEventListener(['a', 'b'], $y);
$em->addEventListener('a', $z);
$em->addEventListener(['a'], $x);
$em->dispatchEvent('a');
$out['first_dispatch'] = $log;
$log = [];

$em->removeEventListener('a', $x);
$em->addEventListener('a', $x);
$em->dispatchEvent('a');
$out['after_re_adding_x'] = $log;
$log = [];

$em->removeEventListener(['b', 'c'], $y);
$out['events_after_removing_y'] = array_keys($em->getAllListeners());
$out['has_listeners_b'] = $em->hasListeners('b');
$out['count_listeners_b'] = count($em->getListeners('b'));

$property = new ReflectionProperty(EventArgs::class, 'emptyEventArgsInstance');
$property->setValue(null, null);
$em->dispatchEvent('b');
$out['dispatch_b_creates_empty_instance'] = $property->getValue() !== null;

try {
    $em->addEventListener('missing', $x);
    $em->dispatchEvent('missing');
    $out['missing_method'] = null;
} catch (Error $e) {
    $out['missing_method'] = [get_class($e), $e->getMessage()];
}

echo json_encode($out, JSON_THROW_ON_ERROR), "\n";
