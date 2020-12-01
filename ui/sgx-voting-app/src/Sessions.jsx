import React from 'react';
import {Col, Row, Table, Button, Card, Modal, Spinner} from "react-bootstrap";
import 'open-iconic/font/css/open-iconic-bootstrap.min.css';
import './Manager';
import NewVotingDialog from "./modals/NewVoting";

const getJson = async (url) => await (await fetch(url)).json();

class Sessions extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            sessions: [],
            show: false,
            loading: false
        };
    }

    refresh() {
        getJson("/sessions").then((sessions) => this.setState({ sessions, loading: false }));
    }

    async dropSession(session) {
        this.setState({loading: true});
        const response = await fetch(new Request(`/sessions/${session.managerAddress}`, {method: "DELETE"}));
        this.refresh()
    }

    componentDidMount() {
        this.refresh();
    }

    render() {
        const {sessions, show, loading} = this.state;
        const handleClose = () => this.setState({show: false});

        return (
            <Card>
                <Card.Body>
                    { loading ? <Spinner animation="grow"/> : ""}
                    <Table striped bordered hover>
                        <caption>Active Voting Sessions</caption>
                        <thead className="thead-dark">
                        <tr>
                            <th>Adress</th>
                            <th>Contract</th>
                            <th>Voting</th>
                            <th>State</th>
                            <th>&nbsp;</th>
                        </tr>
                        </thead>
                        <tbody>
                        {sessions.map((session) => <tr key={session.managerAddress}>
                            <td>0x{session.managerAddress}</td>
                            <td>0x{session.contract}</td>
                            <td>{session.votingId}</td>
                            <td></td>
                            <td>{loading ? "" : <>
                                <Button variant="danger" size="sm" onClick={() => this.dropSession(session)}><i
                                    className="oi oi-trash"></i></Button>&nbsp; <Button size="sm" href={`#/sessions/${session.managerAddress}`}><i className="oi oi-cog"></i></Button>
                                </>
                            }</td>
                        </tr>)}
                        </tbody>
                    </Table>
                </Card.Body>
                <Card.Footer>
                    <Row>
                        <Col>
                            <Button size="sm" onClick={() => this.setState({show: true})}><i className="oi oi-plus"></i> Create</Button>&nbsp;
                            <Button size="sm" variant="secondary" onClick={() => this.refresh()}><i className="oi oi-reload"></i> Refresh</Button>
                        </Col>
                    </Row>
                </Card.Footer>
                <NewVotingDialog show={show} handleClose={handleClose} handleCreate={() => {
                    this.setState({show: false});
                    this.refresh();
                }}/>
            </Card>);
    }
}

export default Sessions;