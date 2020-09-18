import React from 'react';
import * as Bs from 'react-bootstrap';
import Sessions from "./Sessions";
import Nodes from './Nodes';
import {HashRouter as Router, Switch, Route, useParams} from 'react-router-dom';
import Web3Connect from "./Web3";
import SessionDetails from "./SessionDetails";


function App() {

    return (
        <Router basename="x" >
            <header>
            <Bs.Navbar variant="dark" bg="dark">
                <Bs.Navbar.Brand>Trustless Voting Demo</Bs.Navbar.Brand>
                <Bs.Navbar.Toggle aria-controls="basic-navbar-nav"/>
                <Bs.Navbar.Collapse id="basic-navbar-nav">
                    <Bs.Nav className="mr-auto">
                        <Bs.Nav.Link href="#/sessions"><span className="oi oi-home"></span> Sessions</Bs.Nav.Link>
                        <Bs.Nav.Link href="#/nodes">Nodes</Bs.Nav.Link>
                    </Bs.Nav>
                </Bs.Navbar.Collapse>
                <Bs.Navbar>
                    <Web3Connect/>
                </Bs.Navbar>
            </Bs.Navbar>
            </header>
            <Bs.Container fluid id="main">
                <Bs.Row>
                    <Bs.Col>
                        <Switch>
                            <Route path="/" exact>
                                <Sessions></Sessions>
                            </Route>
                            <Route path="/sessions/:managerAddress">
                                <SessionDetails/>
                            </Route>
                            <Route path="/sessions">
                                <Sessions/>
                            </Route>
                            <Route path="/nodes">
                                <Nodes/>
                            </Route>
                        </Switch>
                    </Bs.Col>
                </Bs.Row>
            </Bs.Container>
        </Router>
    );
}

export default App;
